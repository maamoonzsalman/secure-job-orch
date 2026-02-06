use crate::error::AppError;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: String,
    pub grpc_bind_addr: String,
    pub supabase_url: String,
    pub supabase_service_key: String,
    pub signing_key_hex: String,
    pub compute_path: PathBuf,
    // Optional mTLS configuration (used in Phase 3b)
    #[allow(dead_code)]
    pub grpc_server_cert: Option<PathBuf>,
    #[allow(dead_code)]
    pub grpc_server_key: Option<PathBuf>,
    #[allow(dead_code)]
    pub grpc_ca_cert: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let grpc_bind_addr =
            std::env::var("GRPC_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:50051".to_string());
        let supabase_url = std::env::var("SUPABASE_URL")
            .map_err(|_| AppError::Config("SUPABASE_URL is required".to_string()))?;
        let supabase_service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| AppError::Config("SUPABASE_SERVICE_ROLE_KEY is required".to_string()))?;
        let signing_key_hex = std::env::var("JOB_SIGNING_KEY")
            .map_err(|_| AppError::Config("JOB_SIGNING_KEY is required".to_string()))?;
        let compute_path = std::env::var("COMPUTE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./compute/compute_module"));

        // Optional mTLS configuration
        let grpc_server_cert = std::env::var("GRPC_SERVER_CERT").ok().map(PathBuf::from);
        let grpc_server_key = std::env::var("GRPC_SERVER_KEY").ok().map(PathBuf::from);
        let grpc_ca_cert = std::env::var("GRPC_CA_CERT").ok().map(PathBuf::from);

        Ok(Self {
            bind_addr,
            grpc_bind_addr,
            supabase_url,
            supabase_service_key,
            signing_key_hex,
            compute_path,
            grpc_server_cert,
            grpc_server_key,
            grpc_ca_cert,
        })
    }
}
