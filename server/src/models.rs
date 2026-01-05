use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct JobInput {
    pub job_type: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobRecord {
    pub id: Uuid,
    pub job_type: String,
    pub payload: Value,
    pub state: String,
    pub payload_hash: String,
    pub backend_signature: String,
    pub verification_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub job: JobRecord,
    pub public_key: String,
}

#[derive(Debug, Serialize)]
pub struct JobListResponse {
    pub jobs: Vec<JobRecord>,
    pub public_key: String,
}

#[derive(Debug, Serialize)]
pub struct JobUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}
