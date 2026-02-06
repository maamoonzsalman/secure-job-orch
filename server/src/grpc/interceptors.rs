use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use tonic::{Request, Status};

/// Interceptor that validates Ed25519 signatures from gRPC metadata.
/// 
/// Expected metadata headers:
/// - `x-signature`: Base64-encoded Ed25519 signature
/// - `x-public-key`: Base64-encoded public key
/// - `x-message`: The message that was signed (for verification)
/// 
/// If all three are present, the signature is verified.
/// If none are present, the request passes through (for unauthenticated endpoints).
/// If only some are present, the request is rejected.
pub fn signature_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    let signature_b64 = req.metadata().get("x-signature").and_then(|v| v.to_str().ok());
    let public_key_b64 = req.metadata().get("x-public-key").and_then(|v| v.to_str().ok());
    let message = req.metadata().get("x-message").and_then(|v| v.to_str().ok());

    match (signature_b64, public_key_b64, message) {
        // All present: verify signature
        (Some(sig), Some(pk), Some(msg)) => {
            verify_ed25519_signature(sig, pk, msg)?;
            Ok(req)
        }
        // None present: allow unauthenticated request
        (None, None, None) => Ok(req),
        // Partial: reject as malformed
        _ => Err(Status::invalid_argument(
            "signature verification requires x-signature, x-public-key, and x-message headers",
        )),
    }
}

/// Verify an Ed25519 signature
fn verify_ed25519_signature(
    signature_b64: &str,
    public_key_b64: &str,
    message: &str,
) -> Result<(), Status> {
    // Decode public key
    let pk_bytes = base64::engine::general_purpose::STANDARD
        .decode(public_key_b64)
        .map_err(|e| Status::invalid_argument(format!("invalid public key encoding: {e}")))?;

    let pk_array: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| Status::invalid_argument("public key must be 32 bytes"))?;

    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| Status::invalid_argument(format!("invalid public key: {e}")))?;

    // Decode signature
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| Status::invalid_argument(format!("invalid signature encoding: {e}")))?;

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| Status::invalid_argument(format!("invalid signature: {e}")))?;

    // Verify
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| Status::unauthenticated("signature verification failed"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_signature_verification() {
        // Generate a keypair
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        // Sign a message
        let message = "test message";
        let signature = signing_key.sign(message.as_bytes());

        // Encode for transmission
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let pk_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

        // Verify
        assert!(verify_ed25519_signature(&sig_b64, &pk_b64, message).is_ok());

        // Wrong message should fail
        assert!(verify_ed25519_signature(&sig_b64, &pk_b64, "wrong message").is_err());
    }
}
