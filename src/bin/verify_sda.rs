use m_v_r_esprint1::sced_offer_chain::CanonicalTruthPackage;
use m_v_r_esprint1::sovereign_diagnostic::{verify_diagnostic, DiagnosticVerificationOutcome, SovereignDiagnostic};
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        return Err(
            "Usage: cargo run --bin verify_sda -- <canonical_truth.json> <diagnostic.bin> <public_key_hex32> <approved_firmware_hash_hex32[,hex32..]>".to_string(),
        );
    }

    let truth: CanonicalTruthPackage = serde_json::from_str(
        &fs::read_to_string(&args[1]).map_err(|e| format!("read truth package failed: {e}"))?,
    )
    .map_err(|e| format!("parse truth package failed: {e}"))?;

    let diagnostic_bytes = fs::read(&args[2]).map_err(|e| format!("read diagnostic failed: {e}"))?;
    let diagnostic: SovereignDiagnostic =
        bincode::deserialize(&diagnostic_bytes).map_err(|e| format!("decode diagnostic failed: {e}"))?;

    let pub_key_bytes = hex::decode(&args[3]).map_err(|e| format!("invalid public key hex: {e}"))?;
    if pub_key_bytes.len() != 32 {
        return Err("public key must be 32 bytes (64 hex chars)".to_string());
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pub_key_bytes);
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk)
        .map_err(|e| format!("invalid Ed25519 public key: {e}"))?;

    let mut approved = Vec::new();
    for token in args[4].split(',').filter(|s| !s.trim().is_empty()) {
        let bytes = hex::decode(token.trim()).map_err(|e| format!("invalid firmware hash hex '{token}': {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("firmware hash '{token}' must be 32 bytes (64 hex chars)"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        approved.push(arr);
    }

    let outcome = verify_diagnostic(
        &diagnostic,
        &truth.canonical_payload_bytes,
        &verifying_key,
        &approved,
    );

    match outcome {
        DiagnosticVerificationOutcome::Valid => {
            println!("VALID");
            println!("dtc={}", diagnostic.dtc);
            Ok(())
        }
        DiagnosticVerificationOutcome::InvalidSignature => Err("Invalid Signature".to_string()),
        DiagnosticVerificationOutcome::HashMismatch => Err("Hash Mismatch".to_string()),
        DiagnosticVerificationOutcome::UnrecognizedFirmware => Err("Unrecognized Firmware".to_string()),
    }
}
