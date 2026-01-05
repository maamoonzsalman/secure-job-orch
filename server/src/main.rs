mod compute;
mod config;
mod crypto;
mod error;
mod models;
mod supabase;

use axum::{extract::Path, extract::State, routing::get, routing::post, Json, Router};
use base64::Engine;
use chrono::Utc;
use compute::{checksum_fnv1a64, expected_result, run_compute};
use config::Config;
use crypto::{canonical_json, hash_payload, sign_metadata, verify_metadata};
use ed25519_dalek::{SigningKey, VerifyingKey};
use error::AppError;
use models::{JobInput, JobListResponse, JobRecord, JobResponse, JobUpdate};
use supabase::SupabaseClient;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    supabase: SupabaseClient,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    public_key_b64: String,
    compute_path: std::path::PathBuf,
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
        config.supabase_service_key,
    )?;

    let state = AppState {
        supabase,
        signing_key,
        verifying_key,
        public_key_b64,
        compute_path: config.compute_path,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/jobs", post(create_job).get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|err| AppError::Config(err.to_string()))?;
    println!("Secure Job Orchestrator listening on {}", config.bind_addr);
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::Unexpected(err.to_string()))?;
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

    let job = state.supabase.insert_job(&job).await?;

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
    Path(job_id): Path<Uuid>,
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
