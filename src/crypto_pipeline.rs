// Copyright © 2026 OBINNA JAMES EJIOFOR
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

//! Cryptographic Input Binding Pipeline
//!
//! Ensures all inputs are cryptographically bound into SovereignTrace
//! before normalization, execution, or translation.

use crate::failure_axis::{FailureAxis, SystemHalt};
use crate::sensor_attestation::{sensor_signature_payload, SensorAttestationRegistry};
use crate::sovereign_bus::{SovereignMessage, ActorId, ActorRole, OriginLanguage, TraceId};
use crate::sovereign_trace::{InputEnvelope, Hash256, SovereignTraceLog};
use crate::universal_frontend::{IRModule, Value};
use crate::ir_codegen::{IRInput, IRResult};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Cryptographic pipeline for processing inputs
pub struct CryptoPipeline {
    trace_log: SovereignTraceLog,
    sensor_registry: SensorAttestationRegistry,
    seen_field_packets: HashSet<String>,
}

impl CryptoPipeline {
    pub fn new() -> Self {
        Self {
            trace_log: SovereignTraceLog::new(),
            sensor_registry: SensorAttestationRegistry::new(),
            seen_field_packets: HashSet::new(),
        }
    }

    pub fn with_sensor_registry(sensor_registry: SensorAttestationRegistry) -> Self {
        Self {
            trace_log: SovereignTraceLog::new(),
            sensor_registry,
            seen_field_packets: HashSet::new(),
        }
    }

    pub fn register_field_sensor_deterministic(&mut self, sensor_id: &str) {
        self.sensor_registry.register_deterministic_sensor(sensor_id);
    }

    /// Process raw input through the complete cryptographic pipeline
    pub fn process_input(
        &mut self,
        actor_id: ActorId,
        role: ActorRole,
        origin_language: OriginLanguage,
        raw_input: Vec<u8>,
        signature: Option<Vec<u8>>,
        trace_parent: TraceId,
    ) -> Result<String, SystemHalt> {
        // 1. Create input envelope
        let mut envelope = InputEnvelope::new(
            actor_id.clone(),
            role,
            origin_language.clone(),
            raw_input,
            signature,
            trace_parent,
        );

        // 2. Verify signature/attestation.
        if envelope.role == ActorRole::FieldDevice {
            self.verify_field_device_attestation(&envelope)?;
        } else {
            envelope.verify_signature()?;
        }

        // 3. Record input received
        self.trace_log.append_input_received(envelope.clone());

        // 4. Send envelope to bus
        let bus_message = SovereignMessage::new_input_envelope(envelope.clone());
        if let Some(bus) = &mut *global_bus() {
            bus.send(bus_message);
        }

        // 5. Normalize to IR (placeholder - would call frontend)
        let ir_module = self.normalize_to_ir(&envelope)?;
        let ir_hash = self.hash_ir(&ir_module);
        envelope.set_ir_hash(ir_hash);

        // 6. Record IR normalized
        let raw_hash_str = hex::encode(envelope.raw_input_hash);
        let ir_hash_str = hex::encode(ir_hash);
        self.trace_log.append_ir_normalized(raw_hash_str.clone(), ir_hash_str.clone());

        // 7. Generate Rust code
        let rust_code = crate::ir_codegen::generate_rust_code(&ir_module);
        let rust_hash = self.hash_string(&rust_code);

        // 8. Record Rust generated
        self.trace_log.append_rust_generated(ir_hash_str.clone(), rust_hash.clone());

        // 9. Execute (placeholder)
        let input = IRInput { args: std::collections::HashMap::new() };
        let result = self.execute_rust(&rust_code, input)?;
        let result_hash = self.hash_result(&result);

        // 10. Record execution result
        self.trace_log.append_execution_result(rust_hash.clone(), result_hash.clone());

        // 11. Translate output (placeholder)
        let output = self.translate_output(&result, &origin_language)?;
        let output_hash = self.hash_string(&output);

        // 12. Record output translated
        self.trace_log.append_output_translated(result_hash, output_hash);

        Ok(output)
    }

    /// Normalize raw input to IR (placeholder implementation)
    fn normalize_to_ir(&self, _envelope: &InputEnvelope) -> Result<IRModule, SystemHalt> {
        // In practice, this would use the appropriate frontend based on origin_language
        // For now, return a dummy IR module
        Ok(IRModule {
            functions: vec![],
            constants: vec![],
        })
    }

    /// Hash an IR module
    fn hash_ir(&self, ir: &IRModule) -> Hash256 {
        let mut hasher = Sha256::new();
        // Serialize IR to bytes and hash (simplified)
        hasher.update(format!("{:?}", ir).as_bytes());
        hasher.finalize().into()
    }

    /// Hash a string
    fn hash_string(&self, s: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Hash an execution result
    fn hash_result(&self, result: &IRResult) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", result.value).as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Execute generated Rust code (placeholder)
    fn execute_rust(&self, _rust_code: &str, _input: IRInput) -> Result<IRResult, SystemHalt> {
        // In practice, this would compile and execute the Rust code
        Ok(IRResult {
            value: Value::Int(42),
            bus_messages: vec![],
        })
    }

    /// Translate result back to origin language (placeholder)
    fn translate_output(&self, result: &IRResult, origin_language: &OriginLanguage) -> Result<String, SystemHalt> {
        // In practice, this would use the appropriate backend
        match origin_language {
            OriginLanguage::Python => Ok(format!("return {}", match &result.value {
                Value::Int(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::String(s) => format!("\"{}\"", s),
            })),
            _ => Ok("output".to_string()),
        }
    }

    /// Get access to the trace log
    pub fn trace_log(&self) -> &SovereignTraceLog {
        &self.trace_log
    }

    fn verify_field_device_attestation(&mut self, envelope: &InputEnvelope) -> Result<(), SystemHalt> {
        let signature = envelope.signature.as_deref().ok_or_else(|| {
            SystemHalt::new(
                FailureAxis::SensorAttestationFailure,
                "ERR_SENSOR_SIGNATURE_REQUIRED",
            )
        })?;

        let packet_id = field_packet_id(envelope);
        if !self.seen_field_packets.insert(packet_id) {
            return Err(SystemHalt::new(
                FailureAxis::SensorAttestationFailure,
                "ERR_SENSOR_REPLAY_DETECTED",
            ));
        }

        let payload = sensor_signature_payload(
            &envelope.actor_id.0,
            &envelope.trace_parent.0,
            envelope.timestamp,
            &envelope.raw_input_bytes,
        );

        self.sensor_registry
            .verify_signed_packet(&envelope.actor_id.0, &payload, signature)
    }
}

/// Get global bus instance
fn global_bus() -> std::sync::MutexGuard<'static, Option<crate::sovereign_bus::SovereignBus>> {
    crate::sovereign_bus::global_bus()
}

fn field_packet_id(envelope: &InputEnvelope) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"SENSOR_PACKET_ID_V1");
    hasher.update(envelope.actor_id.0.as_bytes());
    hasher.update(envelope.timestamp.to_le_bytes());
    hasher.update(envelope.trace_parent.0.as_bytes());
    hasher.update(&envelope.raw_input_hash);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor_attestation::{deterministic_sign_for_sensor, sensor_signature_payload};

    #[test]
    fn field_device_requires_valid_signature() {
        let sensor_id = "rtu-west-001";
        let trace = TraceId("trace-a".to_string());
        let raw = b"v=120.1".to_vec();

        let payload = sensor_signature_payload(sensor_id, &trace.0, 0, &raw);
        let signature = deterministic_sign_for_sensor(sensor_id, &payload);

        let mut registry = SensorAttestationRegistry::new();
        registry.register_deterministic_sensor(sensor_id);
        let mut pipeline = CryptoPipeline::with_sensor_registry(registry);

        let result = pipeline.process_input(
            ActorId(sensor_id.to_string()),
            ActorRole::FieldDevice,
            OriginLanguage::Rust,
            raw,
            Some(signature),
            trace,
        );

        // timestamp inside envelope is runtime-generated, so this synthetic signature must fail.
        assert!(result.is_err());
    }

    #[test]
    fn field_device_rejects_missing_signature() {
        let mut registry = SensorAttestationRegistry::new();
        registry.register_deterministic_sensor("pmu-east-01");
        let mut pipeline = CryptoPipeline::with_sensor_registry(registry);

        let err = pipeline
            .process_input(
                ActorId("pmu-east-01".to_string()),
                ActorRole::FieldDevice,
                OriginLanguage::Rust,
                b"hz=60.02".to_vec(),
                None,
                TraceId("trace-x".to_string()),
            )
            .expect_err("must reject unsigned field packet");

        assert_eq!(err.message, "ERR_SENSOR_SIGNATURE_REQUIRED");
    }
}
