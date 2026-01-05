use crate::error::AppError;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: String,
    pub supabase_url: String,
    pub supabase_service_key: String,
    pub signing_key_hex: String,
    pub compute_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let supabase_url = std::env::var("SUPABASE_URL")
            .map_err(|_| AppError::Config("SUPABASE_URL is required".to_string()))?;
        let supabase_service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| AppError::Config("SUPABASE_SERVICE_ROLE_KEY is required".to_string()))?;
        let signing_key_hex = std::env::var("JOB_SIGNING_KEY")
            .map_err(|_| AppError::Config("JOB_SIGNING_KEY is required".to_string()))?;
        let compute_path = std::env::var("COMPUTE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./compute/compute_module"));

        Ok(Self {
            bind_addr,
            supabase_url,
            supabase_service_key,
            signing_key_hex,
            compute_path,
        })
    }
}
