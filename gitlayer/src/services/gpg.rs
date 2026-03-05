//! GPG service implementation for signature verification

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, error, warn};

use crate::config::Config;
use crate::proto::*;

pub struct GpgServiceImpl {
    #[allow(dead_code)]
    config: Arc<Config>,
}

impl GpgServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Run a GPG command with a temporary GNUPGHOME
    fn run_gpg_command(
        args: &[&str],
        stdin_data: Option<&str>,
        import_keys: &[&str],
    ) -> Result<String, Status> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            Status::internal("Failed to create GPG environment")
        })?;

        let gpg_home = temp_dir.path();

        // Import keys if provided
        for key in import_keys {
            let mut import_cmd = Command::new("gpg")
                .args(["--homedir", gpg_home.to_str().unwrap(), "--batch", "--import"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| {
                    Status::internal(format!("Failed to run gpg: {}", e))
                })?;

            if let Some(ref mut stdin) = import_cmd.stdin {
                let _ = stdin.write_all(key.as_bytes());
            }
            let _ = import_cmd.wait();
        }

        // Build the command
        let mut cmd_args = vec!["--homedir", gpg_home.to_str().unwrap(), "--batch"];
        cmd_args.extend_from_slice(args);

        let mut cmd = Command::new("gpg")
            .args(&cmd_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Status::internal(format!("Failed to run gpg: {}", e))
            })?;

        if let Some(data) = stdin_data {
            if let Some(ref mut stdin) = cmd.stdin {
                let _ = stdin.write_all(data.as_bytes());
            }
        }

        let output = cmd.wait_with_output().map_err(|e| {
            Status::internal(format!("Failed to wait for gpg: {}", e))
        })?;

        // Return stderr for verification commands (gpg outputs status there)
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if output.status.success() || !stdout.is_empty() {
            Ok(stdout)
        } else {
            Ok(stderr)
        }
    }

    /// Extract key ID from a signature
    fn extract_key_id_from_signature(signature: &str) -> Option<String> {
        // GPG signature typically contains the key ID
        // We can extract it by parsing the signature or using gpg --list-packets
        let temp_dir = tempfile::tempdir().ok()?;
        let gpg_home = temp_dir.path();

        let mut cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str()?,
                "--batch",
                "--list-packets",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        if let Some(ref mut stdin) = cmd.stdin {
            let _ = stdin.write_all(signature.as_bytes());
        }

        let output = cmd.wait_with_output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse output for key ID
        for line in stdout.lines() {
            if line.contains("keyid") {
                if let Some(idx) = line.find("keyid") {
                    let rest = &line[idx + 6..];
                    let key_id: String = rest.trim().chars().take_while(|c| c.is_alphanumeric()).collect();
                    if !key_id.is_empty() {
                        return Some(key_id);
                    }
                }
            }
        }

        None
    }

    /// Parse GPG colon output for key info
    fn parse_key_info(output: &str) -> GetKeyInfoResponse {
        let mut key_id = String::new();
        let mut fingerprint = String::new();
        let mut algorithm = String::new();
        let mut key_size: i32 = 0;
        let mut created_at: i64 = 0;
        let mut expires_at: i64 = 0;
        let mut emails = Vec::new();
        let mut can_sign = false;
        let mut can_encrypt = false;
        let mut can_certify = false;

        for line in output.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.is_empty() {
                continue;
            }

            match fields[0] {
                "pub" => {
                    if fields.len() > 11 {
                        key_size = fields[2].parse().unwrap_or(0);
                        algorithm = Self::algorithm_from_code(fields[3]);
                        key_id = fields[4].to_string();
                        created_at = fields[5].parse().unwrap_or(0);
                        expires_at = fields[6].parse().unwrap_or(0);

                        if fields.len() > 11 {
                            let caps = fields[11];
                            can_sign = caps.contains('s') || caps.contains('S');
                            can_encrypt = caps.contains('e') || caps.contains('E');
                            can_certify = caps.contains('c') || caps.contains('C');
                        }
                    }
                }
                "fpr" => {
                    if fields.len() > 9 && fingerprint.is_empty() {
                        fingerprint = fields[9].to_string();
                    }
                }
                "uid" => {
                    if fields.len() > 9 {
                        let uid = fields[9];
                        if let Some(start) = uid.find('<') {
                            if let Some(end) = uid.find('>') {
                                let email = uid[start + 1..end].to_string();
                                if !emails.contains(&email) {
                                    emails.push(email);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        GetKeyInfoResponse {
            success: !fingerprint.is_empty(),
            key_id,
            fingerprint,
            algorithm,
            key_size,
            created_at,
            expires_at,
            emails,
            can_sign,
            can_encrypt,
            can_certify,
            error_message: String::new(),
        }
    }

    /// Convert algorithm code to name
    fn algorithm_from_code(code: &str) -> String {
        match code {
            "1" | "2" | "3" => "RSA".to_string(),
            "16" => "Elgamal".to_string(),
            "17" => "DSA".to_string(),
            "18" => "ECDH".to_string(),
            "19" => "ECDSA".to_string(),
            "22" => "EdDSA".to_string(),
            _ => format!("Unknown({})", code),
        }
    }
}

#[tonic::async_trait]
impl gpg_service_server::GpgService for GpgServiceImpl {
    async fn verify_signature(
        &self,
        request: Request<VerifySignatureRequest>,
    ) -> Result<Response<VerifySignatureResponse>, Status> {
        let req = request.into_inner();
        debug!(
            "Verifying GPG signature, data length: {}, sig length: {}",
            req.signed_data.len(),
            req.signature.len()
        );

        if req.signature.is_empty() {
            return Ok(Response::new(VerifySignatureResponse {
                status: GpgVerificationStatus::GpgStatusNoSignature as i32,
                message: "No signature provided".to_string(),
                ..Default::default()
            }));
        }

        // Extract key ID from signature
        let key_id = Self::extract_key_id_from_signature(&req.signature).unwrap_or_default();

        // If no trusted keys provided, we can only check if signature format is valid
        if req.trusted_keys.is_empty() {
            return Ok(Response::new(VerifySignatureResponse {
                status: GpgVerificationStatus::GpgStatusUnknownKey as i32,
                message: "No trusted keys provided for verification".to_string(),
                key_id,
                ..Default::default()
            }));
        }

        // Create temporary GPG home and import trusted keys
        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            Status::internal("Failed to create GPG environment")
        })?;

        let gpg_home = temp_dir.path();

        // Import all trusted keys
        for key in &req.trusted_keys {
            let mut import_cmd = Command::new("gpg")
                .args([
                    "--homedir",
                    gpg_home.to_str().unwrap(),
                    "--batch",
                    "--import",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| Status::internal(format!("Failed to run gpg: {}", e)))?;

            if let Some(ref mut stdin) = import_cmd.stdin {
                let _ = stdin.write_all(key.as_bytes());
            }
            let _ = import_cmd.wait();
        }

        // Create temporary files for signature and data
        let sig_file = temp_dir.path().join("sig.asc");
        let data_file = temp_dir.path().join("data");

        std::fs::write(&sig_file, &req.signature).map_err(|e| {
            Status::internal(format!("Failed to write signature file: {}", e))
        })?;
        std::fs::write(&data_file, &req.signed_data).map_err(|e| {
            Status::internal(format!("Failed to write data file: {}", e))
        })?;

        // Verify the signature
        let verify_output = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--status-fd",
                "1",
                "--verify",
                sig_file.to_str().unwrap(),
                data_file.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| Status::internal(format!("Failed to run gpg verify: {}", e)))?;

        let stdout = String::from_utf8_lossy(&verify_output.stdout);
        let stderr = String::from_utf8_lossy(&verify_output.stderr);

        debug!("GPG verify stdout: {}", stdout);
        debug!("GPG verify stderr: {}", stderr);

        // Parse verification result
        let (status, message, signer_name, signer_email) = if stdout.contains("GOODSIG") {
            // Extract signer info from GOODSIG line
            let mut name = String::new();
            let mut email = String::new();

            for line in stdout.lines() {
                if line.contains("GOODSIG") {
                    // Format: [GNUPG:] GOODSIG <keyid> <name> <email>
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let uid_parts: Vec<&str> = parts[3..].iter().copied().collect();
                        let uid = uid_parts.join(" ");
                        if let Some(start) = uid.find('<') {
                            name = uid[..start].trim().to_string();
                            if let Some(end) = uid.find('>') {
                                email = uid[start + 1..end].to_string();
                            }
                        } else {
                            name = uid;
                        }
                    }
                }
            }

            // Check if email matches committer (if provided)
            if !req.committer_email.is_empty() && !email.is_empty() {
                if email.to_lowercase() != req.committer_email.to_lowercase() {
                    (
                        GpgVerificationStatus::GpgStatusBadEmail,
                        format!("Key email '{}' doesn't match committer '{}'", email, req.committer_email),
                        name,
                        email,
                    )
                } else {
                    (
                        GpgVerificationStatus::GpgStatusVerified,
                        "Signature verified".to_string(),
                        name,
                        email,
                    )
                }
            } else {
                (
                    GpgVerificationStatus::GpgStatusVerified,
                    "Signature verified".to_string(),
                    name,
                    email,
                )
            }
        } else if stdout.contains("BADSIG") {
            (
                GpgVerificationStatus::GpgStatusBadSignature,
                "Invalid signature".to_string(),
                String::new(),
                String::new(),
            )
        } else if stdout.contains("ERRSIG") {
            // Check for specific error reasons
            if stdout.contains("EXPKEYSIG") {
                (
                    GpgVerificationStatus::GpgStatusExpiredKey,
                    "Signing key has expired".to_string(),
                    String::new(),
                    String::new(),
                )
            } else if stdout.contains("REVKEYSIG") {
                (
                    GpgVerificationStatus::GpgStatusRevokedKey,
                    "Signing key has been revoked".to_string(),
                    String::new(),
                    String::new(),
                )
            } else {
                (
                    GpgVerificationStatus::GpgStatusUnknownKey,
                    "Cannot verify: unknown key".to_string(),
                    String::new(),
                    String::new(),
                )
            }
        } else if stdout.contains("NO_PUBKEY") || stderr.contains("public key not found") {
            (
                GpgVerificationStatus::GpgStatusUnknownKey,
                "Signing key not found".to_string(),
                String::new(),
                String::new(),
            )
        } else {
            (
                GpgVerificationStatus::GpgStatusUnverified,
                format!("Unable to verify signature: {}", stderr),
                String::new(),
                String::new(),
            )
        };

        Ok(Response::new(VerifySignatureResponse {
            status: status as i32,
            message,
            key_id,
            fingerprint: String::new(), // Would need additional parsing
            signer_name,
            signer_email,
            signature_timestamp: 0, // Would need additional parsing
        }))
    }

    async fn sign(
        &self,
        request: Request<SignRequest>,
    ) -> Result<Response<SignResponse>, Status> {
        let req = request.into_inner();
        debug!("Signing data with GPG, data length: {}", req.data.len());

        if req.private_key.is_empty() {
            return Ok(Response::new(SignResponse {
                success: false,
                signature: String::new(),
                error_message: "No private key provided".to_string(),
            }));
        }

        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            Status::internal("Failed to create GPG environment")
        })?;

        let gpg_home = temp_dir.path();

        // Import the private key
        let mut import_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--import",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to run gpg: {}", e)))?;

        if let Some(ref mut stdin) = import_cmd.stdin {
            let _ = stdin.write_all(req.private_key.as_bytes());
        }
        let _ = import_cmd.wait();

        // Sign the data
        let mut sign_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--armor",
                "--detach-sign",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to run gpg: {}", e)))?;

        if let Some(ref mut stdin) = sign_cmd.stdin {
            let _ = stdin.write_all(req.data.as_bytes());
        }

        let sign_output = sign_cmd.wait_with_output().map_err(|e| {
            Status::internal(format!("Failed to wait for gpg: {}", e))
        })?;

        if sign_output.status.success() {
            Ok(Response::new(SignResponse {
                success: true,
                signature: String::from_utf8_lossy(&sign_output.stdout).to_string(),
                error_message: String::new(),
            }))
        } else {
            let stderr = String::from_utf8_lossy(&sign_output.stderr);
            warn!("GPG signing failed: {}", stderr);
            Ok(Response::new(SignResponse {
                success: false,
                signature: String::new(),
                error_message: stderr.to_string(),
            }))
        }
    }

    async fn get_key_info(
        &self,
        request: Request<GetKeyInfoRequest>,
    ) -> Result<Response<GetKeyInfoResponse>, Status> {
        let req = request.into_inner();
        debug!("Getting key info, key length: {}", req.public_key.len());

        if req.public_key.is_empty() {
            return Ok(Response::new(GetKeyInfoResponse {
                success: false,
                error_message: "No public key provided".to_string(),
                ..Default::default()
            }));
        }

        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            Status::internal("Failed to create GPG environment")
        })?;

        let gpg_home = temp_dir.path();

        // Import the key
        let mut import_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--import",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to run gpg: {}", e)))?;

        if let Some(ref mut stdin) = import_cmd.stdin {
            let _ = stdin.write_all(req.public_key.as_bytes());
        }
        let _ = import_cmd.wait();

        // List the key with colon output
        let list_output = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--with-colons",
                "--with-fingerprint",
                "--list-keys",
            ])
            .output()
            .map_err(|e| Status::internal(format!("Failed to list keys: {}", e)))?;

        let output = String::from_utf8_lossy(&list_output.stdout);
        Ok(Response::new(Self::parse_key_info(&output)))
    }
}
