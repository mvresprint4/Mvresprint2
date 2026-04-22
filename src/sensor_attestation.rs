// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System,
// including TLBSS geometry, the Universal Execution Layer, the
// Deterministic IR, Rust Codegen Pipeline, SovereignBus, and the
// Cryptographic Audit Chain.
//
// No part of this file, its algorithms, structures, or designs may be
// copied, reproduced, modified, distributed, published, sublicensed,
// reverse-engineered, or used to create derivative works without the
// express written permission of OBINNA JAMES EJIOFOR.
//
// This software contains proprietary trade secrets and confidential
// intellectual property. Unauthorized use is strictly prohibited.

#![deny(unsafe_code)]

use crate::failure_axis::{FailureAxis, SystemHalt};
use ed25519_dalek::{Signature as Ed25519Signature, Signer as DalekSigner, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct SensorAttestationRegistry {
    keys: BTreeMap<String, VerifyingKey>,
}

impl SensorAttestationRegistry {
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
        }
    }

    pub fn register_sensor_key(
        &mut self,
        sensor_id: impl Into<String>,
        verifying_key: VerifyingKey,
    ) {
        self.keys.insert(sensor_id.into(), verifying_key);
    }

    pub fn register_deterministic_sensor(&mut self, sensor_id: &str) {
        self.register_sensor_key(sensor_id.to_string(), deterministic_sensor_keypair(sensor_id).verifying_key());
    }

    pub fn verify_signed_packet(
        &self,
        sensor_id: &str,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<(), SystemHalt> {
        let verifying_key = self.keys.get(sensor_id).ok_or_else(|| {
            SystemHalt::new(
                FailureAxis::SensorAttestationFailure,
                "ERR_SENSOR_UNREGISTERED",
            )
        })?;

        if signature.len() != 64 {
            return Err(SystemHalt::new(
                FailureAxis::SensorAttestationFailure,
                "ERR_SENSOR_SIGNATURE_LENGTH",
            ));
        }

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let signature = Ed25519Signature::from_bytes(&sig_bytes);
        verifying_key.verify(payload, &signature).map_err(|_| {
            SystemHalt::new(
                FailureAxis::SensorAttestationFailure,
                "ERR_SENSOR_SIGNATURE_INVALID",
            )
        })
    }
}

pub fn sensor_signature_payload(
    sensor_id: &str,
    trace_parent: &str,
    timestamp_ms: u64,
    raw_input_bytes: &[u8],
) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"SENSOR_PACKET_ATTESTATION_V1");
    hasher.update((sensor_id.len() as u32).to_le_bytes());
    hasher.update(sensor_id.as_bytes());
    hasher.update((trace_parent.len() as u32).to_le_bytes());
    hasher.update(trace_parent.as_bytes());
    hasher.update(timestamp_ms.to_le_bytes());
    hasher.update((raw_input_bytes.len() as u32).to_le_bytes());
    hasher.update(raw_input_bytes);
    hasher.finalize().to_vec()
}

pub fn deterministic_sign_for_sensor(sensor_id: &str, payload: &[u8]) -> Vec<u8> {
    deterministic_sensor_keypair(sensor_id).sign(payload).to_bytes().to_vec()
}

fn deterministic_sensor_keypair(sensor_id: &str) -> SigningKey {
    let mut hasher = Sha256::new();
    hasher.update(b"mvre-sensor-attestation-v1");
    hasher.update(sensor_id.as_bytes());
    let seed: [u8; 32] = hasher.finalize().into();
    SigningKey::from_bytes(&seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_registered_sensor_signature() {
        let sensor_id = "rtu-west-001";
        let payload = sensor_signature_payload(sensor_id, "trace-1", 1234, b"mw=100.1");
        let signature = deterministic_sign_for_sensor(sensor_id, &payload);

        let mut registry = SensorAttestationRegistry::new();
        registry.register_deterministic_sensor(sensor_id);

        registry
            .verify_signed_packet(sensor_id, &payload, &signature)
            .expect("should verify");
    }

    #[test]
    fn rejects_tampered_sensor_payload() {
        let sensor_id = "pmu-north-002";
        let payload = sensor_signature_payload(sensor_id, "trace-1", 1234, b"hz=60.01");
        let signature = deterministic_sign_for_sensor(sensor_id, &payload);
        let mut tampered = payload.clone();
        tampered[0] ^= 0xAA;

        let mut registry = SensorAttestationRegistry::new();
        registry.register_deterministic_sensor(sensor_id);

        let err = registry
            .verify_signed_packet(sensor_id, &tampered, &signature)
            .expect_err("tampered payload must fail");
        assert_eq!(err.message, "ERR_SENSOR_SIGNATURE_INVALID");
    }
}

