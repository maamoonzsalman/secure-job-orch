use crate::error::AppError;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use std::path::Path;
use std::process::Stdio;

#[derive(Debug, Serialize)]
struct ComputeRequest<'a> {
    job_type: &'a str,
    payload_b64: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ComputeOutput {
    pub result: String,
    pub checksum: String,
    pub algorithm: String,
}

pub async fn run_compute(binary_path: &Path, job_type: &str, payload_b64: &str) -> Result<ComputeOutput, AppError> {
    let request = ComputeRequest { job_type, payload_b64 };
    let input = serde_json::to_vec(&request)?;

    let mut child = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| AppError::Compute(err.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&input).await?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|err| AppError::Compute(err.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::Compute(format!("compute module failed: {stderr}")));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| AppError::Compute(err.to_string()))?;
    let parsed: ComputeOutput = serde_json::from_str(stdout.trim())?;
    Ok(parsed)
}

pub fn expected_result(job_type: &str, payload_b64: &str) -> String {
    format!("processed|{}|{}", job_type, payload_b64)
}

pub fn checksum_fnv1a64(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}
