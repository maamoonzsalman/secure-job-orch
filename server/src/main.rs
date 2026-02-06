mod compute;
mod config;
mod crypto;
mod error;
mod events;
mod grpc;
mod models;
mod supabase;

use axum::{extract::Path as AxumPath, extract::State, routing::get, routing::post, Json, Router};
use base64::Engine;
use chrono::Utc;
use compute::{checksum_fnv1a64, expected_result, run_compute};
use config::Config;
use crypto::{canonical_json, hash_payload, sign_metadata, verify_metadata};
use ed25519_dalek::{SigningKey, VerifyingKey};
use error::AppError;
use events::JobEventBus;
use grpc::interceptors::signature_interceptor;
use grpc::proto::job_orchestrator_server::JobOrchestratorServer;
use grpc::service::{GrpcState, JobOrchestratorService};
use models::{JobInput, JobListResponse, JobRecord, JobResponse, JobUpdate};
use supabase::SupabaseClient;
use tonic::transport::{Certificate, Identity, Server as TonicServer, ServerTlsConfig};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    supabase: SupabaseClient,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    public_key_b64: String,
    compute_path: std::path::PathBuf,
    event_bus: JobEventBus,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let signing_key_bytes = hex::decode(&config.signing_key_hex)
        .map_err(|err| AppError::Config(format!("invalid JOB_SIGNING_KEY: {err}")))?;
    let signing_key = SigningKey::from_bytes(
        signing_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| AppError::Config("JOB_SIGNING_KEY must be 32 bytes hex".to_string()))?,
    );
    let verifying_key = VerifyingKey::from(&signing_key);
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    let supabase = SupabaseClient::new(
        format!("{}/rest/v1", config.supabase_url.trim_end_matches('/')),
        config.supabase_service_key.clone(),
    )?;

    // Create shared event bus for real-time streaming
    let event_bus = JobEventBus::new(256);

    // HTTP server state
    let http_state = AppState {
        supabase: supabase.clone(),
        signing_key: signing_key.clone(),
        verifying_key,
        public_key_b64: public_key_b64.clone(),
        compute_path: config.compute_path.clone(),
        event_bus: event_bus.clone(),
    };

    // gRPC server state
    let grpc_state = GrpcState {
        supabase,
        signing_key,
        verifying_key: VerifyingKey::from(&http_state.signing_key),
        public_key_b64,
        compute_path: config.compute_path,
        event_bus,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/jobs", post(create_job).get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .with_state(http_state)
        .layer(cors);

    // Start HTTP server
    let http_addr = config.bind_addr.clone();
    let http_server = async move {
        let listener = tokio::net::TcpListener::bind(&http_addr)
            .await
            .expect("failed to bind HTTP server");
        println!("HTTP server listening on {}", http_addr);
        axum::serve(listener, app)
            .await
            .expect("HTTP server failed");
    };

    // Start gRPC server
    let grpc_addr = config
        .grpc_bind_addr
        .parse()
        .map_err(|e| AppError::Config(format!("invalid GRPC_BIND_ADDR: {e}")))?;

    let grpc_service = JobOrchestratorService::new(grpc_state);

    // Build gRPC service with signature verification interceptor
    let grpc_svc = JobOrchestratorServer::with_interceptor(grpc_service, signature_interceptor);

    // Check for mTLS configuration
    let grpc_server_cert = config.grpc_server_cert.clone();
    let grpc_server_key = config.grpc_server_key.clone();
    let grpc_ca_cert = config.grpc_ca_cert.clone();

    let grpc_server = async move {
        println!("gRPC server listening on {}", grpc_addr);

        let mut server_builder = TonicServer::builder();

        // Configure TLS if certificates are provided
        if let (Some(cert_path), Some(key_path)) = (grpc_server_cert, grpc_server_key) {
            match configure_tls(&cert_path, &key_path, grpc_ca_cert.as_ref()).await {
                Ok(tls_config) => {
                    println!("gRPC server configured with TLS");
                    server_builder = server_builder
                        .tls_config(tls_config)
                        .expect("failed to configure TLS");
                }
                Err(e) => {
                    eprintln!("Warning: TLS configuration failed: {e}. Running without TLS.");
                }
            }
        }

        server_builder
            .add_service(grpc_svc)
            .serve(grpc_addr)
            .await
            .expect("gRPC server failed");
    };

    // Run both servers concurrently
    tokio::select! {
        _ = http_server => {},
        _ = grpc_server => {},
    }

    Ok(())
}

async fn create_job(
    State(state): State<AppState>,
    Json(input): Json<JobInput>,
) -> Result<Json<JobResponse>, AppError> {
    validate_job_type(&input.job_type)?;

    let now = Utc::now().to_rfc3339();
    let job_id = Uuid::new_v4();
    let payload_hash = hash_payload(&input.payload);
    let signature = sign_metadata(&state.signing_key, job_id, &input.job_type, &payload_hash);

    let job = JobRecord {
        id: job_id,
        job_type: input.job_type.clone(),
        payload: input.payload.clone(),
        state: "SUBMITTED".to_string(),
        payload_hash: payload_hash.clone(),
        backend_signature: signature,
        verification_status: "UNVERIFIED".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let _job = state.supabase.insert_job(&job).await?;

    // Publish event for streaming subscribers
    state.event_bus.publish(events::JobEvent {
        job_id,
        previous_state: "".to_string(),
        new_state: "SUBMITTED".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    state
        .supabase
        .update_job(
            job_id,
            &JobUpdate {
                state: Some("VALIDATED".to_string()),
                verification_status: None,
                updated_at: Some(Utc::now().to_rfc3339()),
            },
        )
        .await?;

    state.event_bus.publish(events::JobEvent {
        job_id,
        previous_state: "SUBMITTED".to_string(),
        new_state: "VALIDATED".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    state
        .supabase
        .update_job(
            job_id,
            &JobUpdate {
                state: Some("PROCESSING".to_string()),
                verification_status: None,
                updated_at: Some(Utc::now().to_rfc3339()),
            },
        )
        .await?;

    state.event_bus.publish(events::JobEvent {
        job_id,
        previous_state: "VALIDATED".to_string(),
        new_state: "PROCESSING".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    let payload_canonical = canonical_json(&input.payload);
    let payload_b64 = base64::engine::general_purpose::STANDARD.encode(payload_canonical.as_bytes());

    let compute_output = match run_compute(&state.compute_path, &input.job_type, &payload_b64).await {
        Ok(output) => output,
        Err(err) => {
            state
                .supabase
                .update_job(
                    job_id,
                    &JobUpdate {
                        state: Some("FAILED".to_string()),
                        verification_status: Some("FAILED".to_string()),
                        updated_at: Some(Utc::now().to_rfc3339()),
                    },
                )
                .await?;

            state.event_bus.publish(events::JobEvent {
                job_id,
                previous_state: "PROCESSING".to_string(),
                new_state: "FAILED".to_string(),
                verification_status: "FAILED".to_string(),
                timestamp: Utc::now(),
            });

            return Err(err);
        }
    };

    let expected = expected_result(&input.job_type, &payload_b64);
    let expected_checksum = checksum_fnv1a64(&expected);

    let verification_status = if compute_output.result == expected
        && compute_output.checksum == expected_checksum
        && compute_output.algorithm == "fnv1a-64"
    {
        "VERIFIED"
    } else {
        "INVALID"
    };

    let final_state = if verification_status == "VERIFIED" {
        "VERIFIED"
    } else {
        "FAILED"
    };

    let job = state
        .supabase
        .update_job(
            job_id,
            &JobUpdate {
                state: Some(final_state.to_string()),
                verification_status: Some(verification_status.to_string()),
                updated_at: Some(Utc::now().to_rfc3339()),
            },
        )
        .await?;

    state.event_bus.publish(events::JobEvent {
        job_id,
        previous_state: "PROCESSING".to_string(),
        new_state: final_state.to_string(),
        verification_status: verification_status.to_string(),
        timestamp: Utc::now(),
    });

    if !verify_metadata(
        &state.verifying_key,
        job.id,
        &job.job_type,
        &job.payload_hash,
        &job.backend_signature,
    )? {
        return Err(AppError::Unexpected("signature verification failed".to_string()));
    }

    Ok(Json(JobResponse {
        job,
        public_key: state.public_key_b64.clone(),
    }))
}

async fn list_jobs(State(state): State<AppState>) -> Result<Json<JobListResponse>, AppError> {
    let jobs = state.supabase.list_jobs().await?;
    Ok(Json(JobListResponse {
        jobs,
        public_key: state.public_key_b64.clone(),
    }))
}

async fn get_job(
    State(state): State<AppState>,
    AxumPath(job_id): AxumPath<Uuid>,
) -> Result<Json<JobResponse>, AppError> {
    let job = state
        .supabase
        .get_job(job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("job not found".to_string()))?;

    Ok(Json(JobResponse {
        job,
        public_key: state.public_key_b64.clone(),
    }))
}

fn validate_job_type(job_type: &str) -> Result<(), AppError> {
    let is_valid = !job_type.is_empty()
        && job_type.len() <= 40
        && job_type
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if is_valid {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "job_type must be 1-40 chars, alphanumeric plus - or _".to_string(),
        ))
    }
}

/// Configure TLS for the gRPC server
/// 
/// If `ca_cert_path` is provided, mutual TLS (mTLS) is enabled, requiring
/// clients to present valid certificates signed by the CA.
async fn configure_tls(
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
    ca_cert_path: Option<&std::path::PathBuf>,
) -> Result<ServerTlsConfig, AppError> {
    // Read server certificate and key
    let cert_pem = tokio::fs::read(cert_path)
        .await
        .map_err(|e| AppError::Config(format!("failed to read server cert: {e}")))?;
    let key_pem = tokio::fs::read(key_path)
        .await
        .map_err(|e| AppError::Config(format!("failed to read server key: {e}")))?;

    let identity = Identity::from_pem(cert_pem, key_pem);
    let mut tls_config = ServerTlsConfig::new().identity(identity);

    // If CA cert is provided, enable mutual TLS
    if let Some(ca_path) = ca_cert_path {
        let ca_pem = tokio::fs::read(ca_path)
            .await
            .map_err(|e| AppError::Config(format!("failed to read CA cert: {e}")))?;
        let ca_cert = Certificate::from_pem(ca_pem);
        tls_config = tls_config.client_ca_root(ca_cert);
        println!("mTLS enabled: clients must present valid certificates");
    }

    Ok(tls_config)
}
