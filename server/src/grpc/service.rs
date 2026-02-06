use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

use super::convert::parse_payload_json;
use super::proto::job_orchestrator_server::JobOrchestrator;
use super::proto::*;
use crate::events::{JobEvent, JobEventBus};
use crate::error::AppError;
use crate::models::{JobListResponse as DomainJobListResponse, JobResponse as DomainJobResponse};

/// Shared state for the gRPC service
#[derive(Clone)]
pub struct GrpcState {
    pub supabase: crate::supabase::SupabaseClient,
    pub signing_key: ed25519_dalek::SigningKey,
    pub verifying_key: ed25519_dalek::VerifyingKey,
    pub public_key_b64: String,
    pub compute_path: std::path::PathBuf,
    pub event_bus: JobEventBus,
}

/// The gRPC service implementation
pub struct JobOrchestratorService {
    state: Arc<GrpcState>,
}

impl JobOrchestratorService {
    pub fn new(state: GrpcState) -> Self {
        Self {
            state: Arc::new(state),
        }
    }
}

// Type aliases for streaming responses
type WatchJobStream = Pin<Box<dyn Stream<Item = Result<JobStatusUpdate, Status>> + Send>>;
type WatchAllJobsStream = Pin<Box<dyn Stream<Item = Result<JobStatusUpdate, Status>> + Send>>;
type ProcessJobsStream = Pin<Box<dyn Stream<Item = Result<JobResponse, Status>> + Send>>;

#[tonic::async_trait]
impl JobOrchestrator for JobOrchestratorService {
    // ========================================================================
    // UNARY RPCs (Phase 2b)
    // ========================================================================

    async fn create_job(
        &self,
        request: Request<CreateJobRequest>,
    ) -> Result<Response<JobResponse>, Status> {
        let req = request.into_inner();

        let payload = parse_payload_json(&req.payload_json)
            .map_err(|e| Status::invalid_argument(format!("invalid payload JSON: {e}")))?;

        let response = create_job_internal(&self.state, &req.job_type, payload).await?;

        Ok(Response::new(response.into()))
    }

    async fn get_job(&self, request: Request<GetJobRequest>) -> Result<Response<JobResponse>, Status> {
        let job_id: Uuid = request
            .into_inner()
            .job_id
            .parse()
            .map_err(|_| Status::invalid_argument("invalid job_id format"))?;

        let job = self
            .state
            .supabase
            .get_job(job_id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| Status::not_found("job not found"))?;

        let response = DomainJobResponse {
            job,
            public_key: self.state.public_key_b64.clone(),
        };

        Ok(Response::new(response.into()))
    }

    async fn list_jobs(
        &self,
        _request: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        let jobs = self.state.supabase.list_jobs().await.map_err(AppError::from)?;

        let response = DomainJobListResponse {
            jobs,
            public_key: self.state.public_key_b64.clone(),
        };

        Ok(Response::new(response.into()))
    }

    // ========================================================================
    // SERVER STREAMING (Phase 2c)
    // ========================================================================

    type WatchJobStream = WatchJobStream;

    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let target_job_id: Uuid = request
            .into_inner()
            .job_id
            .parse()
            .map_err(|_| Status::invalid_argument("invalid job_id format"))?;

        let mut receiver = self.state.event_bus.subscribe();

        let (tx, rx) = mpsc::channel(32);

        // Spawn a task to filter and forward events
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) if event.job_id == target_job_id => {
                        let update = JobStatusUpdate {
                            job_id: event.job_id.to_string(),
                            previous_state: event.previous_state,
                            new_state: event.new_state.clone(),
                            verification_status: event.verification_status,
                            timestamp: event.timestamp.to_rfc3339(),
                        };
                        if tx.send(Ok(update)).await.is_err() {
                            break; // Client disconnected
                        }
                        // End stream when job reaches terminal state
                        if event.new_state == "VERIFIED" || event.new_state == "FAILED" {
                            break;
                        }
                    }
                    Ok(_) => {} // Different job, skip
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {} // Skip lagged events
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type WatchAllJobsStream = WatchAllJobsStream;

    async fn watch_all_jobs(
        &self,
        _request: Request<WatchAllJobsRequest>,
    ) -> Result<Response<Self::WatchAllJobsStream>, Status> {
        let mut receiver = self.state.event_bus.subscribe();

        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let update = JobStatusUpdate {
                            job_id: event.job_id.to_string(),
                            previous_state: event.previous_state,
                            new_state: event.new_state,
                            verification_status: event.verification_status,
                            timestamp: event.timestamp.to_rfc3339(),
                        };
                        if tx.send(Ok(update)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    // ========================================================================
    // CLIENT STREAMING (Phase 2d)
    // ========================================================================

    async fn submit_batch(
        &self,
        request: Request<Streaming<CreateJobRequest>>,
    ) -> Result<Response<BatchResponse>, Status> {
        let mut stream = request.into_inner();

        let mut total = 0i32;
        let mut successful = 0i32;
        let mut failed = 0i32;
        let mut job_ids = Vec::new();
        let mut errors = Vec::new();

        while let Some(result) = stream.next().await {
            let req = result?;
            total += 1;

            let payload = match parse_payload_json(&req.payload_json) {
                Ok(p) => p,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("invalid payload JSON: {e}"));
                    continue;
                }
            };

            match create_job_internal(&self.state, &req.job_type, payload).await {
                Ok(response) => {
                    successful += 1;
                    job_ids.push(response.job.id.to_string());
                }
                Err(e) => {
                    failed += 1;
                    errors.push(e.to_string());
                }
            }
        }

        Ok(Response::new(BatchResponse {
            total_submitted: total,
            successful,
            failed,
            job_ids,
            errors,
        }))
    }

    // ========================================================================
    // BIDIRECTIONAL STREAMING (Phase 2e)
    // ========================================================================

    type ProcessJobsStream = ProcessJobsStream;

    async fn process_jobs(
        &self,
        request: Request<Streaming<CreateJobRequest>>,
    ) -> Result<Response<Self::ProcessJobsStream>, Status> {
        let mut incoming = request.into_inner();
        let state = self.state.clone();

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(result) = incoming.next().await {
                let req = match result {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        continue;
                    }
                };

                let payload = match parse_payload_json(&req.payload_json) {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = tx
                            .send(Err(Status::invalid_argument(format!(
                                "invalid payload JSON: {e}"
                            ))))
                            .await;
                        continue;
                    }
                };

                match create_job_internal(&state, &req.job_type, payload).await {
                    Ok(response) => {
                        if tx.send(Ok(response.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(e)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

// ============================================================================
// INTERNAL BUSINESS LOGIC
// ============================================================================

/// Core job creation logic - reused by all RPCs
async fn create_job_internal(
    state: &Arc<GrpcState>,
    job_type: &str,
    payload: serde_json::Value,
) -> Result<DomainJobResponse, Status> {
    use crate::compute::{checksum_fnv1a64, expected_result, run_compute};
    use crate::crypto::{canonical_json, hash_payload, sign_metadata, verify_metadata};
    use crate::models::{JobRecord, JobUpdate};

    // Validate job type
    validate_job_type(job_type)?;

    let now = Utc::now().to_rfc3339();
    let job_id = Uuid::new_v4();
    let payload_hash = hash_payload(&payload);
    let signature = sign_metadata(&state.signing_key, job_id, job_type, &payload_hash);

    let job = JobRecord {
        id: job_id,
        job_type: job_type.to_string(),
        payload: payload.clone(),
        state: "SUBMITTED".to_string(),
        payload_hash: payload_hash.clone(),
        backend_signature: signature,
        verification_status: "UNVERIFIED".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let _job = state.supabase.insert_job(&job).await.map_err(AppError::from)?;

    // Publish SUBMITTED event
    state.event_bus.publish(JobEvent {
        job_id,
        previous_state: "".to_string(),
        new_state: "SUBMITTED".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    // Transition to VALIDATED
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
        .await
        .map_err(AppError::from)?;

    state.event_bus.publish(JobEvent {
        job_id,
        previous_state: "SUBMITTED".to_string(),
        new_state: "VALIDATED".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    // Transition to PROCESSING
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
        .await
        .map_err(AppError::from)?;

    state.event_bus.publish(JobEvent {
        job_id,
        previous_state: "VALIDATED".to_string(),
        new_state: "PROCESSING".to_string(),
        verification_status: "UNVERIFIED".to_string(),
        timestamp: Utc::now(),
    });

    // Run compute module
    let payload_canonical = canonical_json(&payload);
    let payload_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, payload_canonical.as_bytes());

    let compute_output = match run_compute(&state.compute_path, job_type, &payload_b64).await {
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
                .await
                .map_err(AppError::from)?;

            state.event_bus.publish(JobEvent {
                job_id,
                previous_state: "PROCESSING".to_string(),
                new_state: "FAILED".to_string(),
                verification_status: "FAILED".to_string(),
                timestamp: Utc::now(),
            });

            return Err(err.into());
        }
    };

    // Verify compute output
    let expected = expected_result(job_type, &payload_b64);
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
        .await
        .map_err(AppError::from)?;

    state.event_bus.publish(JobEvent {
        job_id,
        previous_state: "PROCESSING".to_string(),
        new_state: final_state.to_string(),
        verification_status: verification_status.to_string(),
        timestamp: Utc::now(),
    });

    // Final signature verification
    if !verify_metadata(
        &state.verifying_key,
        job.id,
        &job.job_type,
        &job.payload_hash,
        &job.backend_signature,
    )
    .map_err(AppError::from)?
    {
        return Err(Status::internal("signature verification failed"));
    }

    Ok(DomainJobResponse {
        job,
        public_key: state.public_key_b64.clone(),
    })
}

fn validate_job_type(job_type: &str) -> Result<(), Status> {
    let is_valid = !job_type.is_empty()
        && job_type.len() <= 40
        && job_type
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');

    if is_valid {
        Ok(())
    } else {
        Err(Status::invalid_argument(
            "job_type must be 1-40 chars, alphanumeric plus - or _",
        ))
    }
}
