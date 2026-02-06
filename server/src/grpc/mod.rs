pub mod convert;
pub mod interceptors;
pub mod service;

// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("secure_job_orch.v1");
}
