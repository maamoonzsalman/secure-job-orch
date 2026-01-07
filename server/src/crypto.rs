use crate::error::AppError;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(val) => val.to_string(),
        serde_json::Value::Number(num) => num.to_string(),
        serde_json::Value::String(val) => serde_json::to_string(val).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::Value::Array(items) => {
            let mut out = String::from("[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                if let Some(value) = map.get(*key) {
                    out.push_str(&canonical_json(value));
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
            out
        }
    }
}

pub fn hash_payload(value: &serde_json::Value) -> String {
    let canonical = canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

pub fn sign_metadata(signing_key: &SigningKey, job_id: Uuid, job_type: &str, payload_hash: &str) -> String {
    let message = format!("{}|{}|{}", job_id, job_type, payload_hash);
    let signature: Signature = signing_key.sign(message.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
}

pub fn verify_metadata(
    verifying_key: &VerifyingKey,
    job_id: Uuid,
    job_type: &str,
    payload_hash: &str,
    signature_b64: &str,
) -> Result<bool, AppError> {
    let message = format!("{}|{}|{}", job_id, job_type, payload_hash);
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|err| AppError::BadRequest(format!("invalid signature encoding: {err}")))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|err| AppError::BadRequest(format!("invalid signature bytes: {err}")))?;
    Ok(verifying_key.verify_strict(message.as_bytes(), &signature).is_ok())
}
