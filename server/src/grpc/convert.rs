use crate::models::{JobRecord as DomainJobRecord, JobResponse as DomainJobResponse, JobListResponse as DomainJobListResponse};
use super::proto::{JobRecord as ProtoJobRecord, JobResponse as ProtoJobResponse, ListJobsResponse as ProtoListJobsResponse};

/// Convert domain JobRecord to proto JobRecord
impl From<DomainJobRecord> for ProtoJobRecord {
    fn from(job: DomainJobRecord) -> Self {
        ProtoJobRecord {
            id: job.id.to_string(),
            job_type: job.job_type,
            payload_json: job.payload.to_string(),
            state: job.state,
            payload_hash: job.payload_hash,
            backend_signature: job.backend_signature,
            verification_status: job.verification_status,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

/// Convert domain JobResponse to proto JobResponse
impl From<DomainJobResponse> for ProtoJobResponse {
    fn from(resp: DomainJobResponse) -> Self {
        ProtoJobResponse {
            job: Some(resp.job.into()),
            public_key: resp.public_key,
        }
    }
}

/// Convert domain JobListResponse to proto ListJobsResponse
impl From<DomainJobListResponse> for ProtoListJobsResponse {
    fn from(resp: DomainJobListResponse) -> Self {
        ProtoListJobsResponse {
            jobs: resp.jobs.into_iter().map(|j| j.into()).collect(),
            public_key: resp.public_key,
        }
    }
}

/// Parse JSON string from proto to serde_json::Value
pub fn parse_payload_json(payload_json: &str) -> Result<serde_json::Value, serde_json::Error> {
    if payload_json.is_empty() {
        Ok(serde_json::Value::Object(serde_json::Map::new()))
    } else {
        serde_json::from_str(payload_json)
    }
}
