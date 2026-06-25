// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// Evidence Repository: Immutable Audit Trail for ISE Deterministic Verification
// Cryptographically sealed evidence chain for timing vs. data corruption classification.
//
// PHASE 4 IMPLEMENTATION:
// - Immutable, hash-linked evidence records
// - Cryptographic fingerprints and attestations
// - Classified failure tracking (timing vs. data)
// - Signature generation for compliance audits

#![deny(unsafe_code)]

use crate::canonical_time::CanonicalTime;
use crate::failure_axis::{FailureAxis, SystemHalt};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::VecDeque;

/// Immutable evidence record in the audit chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub sequence: u64,
    pub timestamp: u128,  // Nanoseconds
    pub parent_hash: Vec<u8>,  // Hash of previous record
    pub record_hash: Vec<u8>,  // Hash of this record's content
    pub failure_class: String,  // Failure classification
    pub evidence: String,  // Diagnostic evidence
    pub signature_placeholder: Vec<u8>,  // TPM signature placeholder
}

impl AuditRecord {
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(&self.parent_hash);
        hasher.update(self.failure_class.as_bytes());
        hasher.update(self.evidence.as_bytes());
        hasher.finalize().to_vec()
    }
}

/// Classification of evidence for deterministic audit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceClass {
    /// Timing within IEEE 1588 tolerances
    TimingOk,
    /// Phase or frequency offset exceeds threshold (HALT_0xABF3)
    TimingDrift,
    /// Parity check or hash mismatch detected
    DataCorruption,
    /// Adversarial fault detected (injected)
    InjectionDetected,
    /// Authority or stratum degradation
    AuthorityInversion,
}

impl EvidenceClass {
    pub fn to_string(&self) -> String {
        match self {
            Self::TimingOk => "TIMING_OK".to_string(),
            Self::TimingDrift => "TIMING_DRIFT".to_string(),
            Self::DataCorruption => "DATA_CORRUPTION".to_string(),
            Self::InjectionDetected => "INJECTION_DETECTED".to_string(),
            Self::AuthorityInversion => "AUTHORITY_INVERSION".to_string(),
        }
    }
}

/// Immutable evidence repository with hash-linking
#[derive(Debug, Serialize, Deserialize)]
pub struct EvidenceRepository {
    records: VecDeque<AuditRecord>,
    root_hash: Vec<u8>,
    total_records: u64,
    last_hash: Vec<u8>,
}

impl EvidenceRepository {
    pub fn new() -> Self {
        let root = Sha256::digest(b"ISE_EVIDENCE_REPOSITORY_ROOT").to_vec();
        Self {
            records: VecDeque::new(),
            root_hash: root.clone(),
            total_records: 0,
            last_hash: root,
        }
    }

    /// Append immutable evidence record
    pub fn append_evidence(
        &mut self,
        timestamp: u128,
        class: EvidenceClass,
        evidence: String,
    ) -> Vec<u8> {
        let record = AuditRecord {
            sequence: self.total_records,
            timestamp,
            parent_hash: self.last_hash.clone(),
            record_hash: Vec::new(),  // Will be filled
            failure_class: class.to_string(),
            evidence,
            signature_placeholder: Vec::new(),
        };

        let computed_hash = record.compute_hash();
        let mut final_record = record;
        final_record.record_hash = computed_hash.clone();

        self.records.push_back(final_record);
        self.total_records += 1;
        self.last_hash = computed_hash.clone();

        computed_hash
    }

    /// Compute repository fingerprint (entire chain hash)
    pub fn compute_fingerprint(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for record in &self.records {
            hasher.update(&record.record_hash);
        }
        hasher.finalize().to_vec()
    }

    /// Verify chain integrity (detect tampering)
    pub fn verify_chain_integrity(&self) -> Result<(), SystemHalt> {
        if self.records.is_empty() {
            return Ok(());
        }

        let mut expected_parent = self.root_hash.clone();

        for (i, record) in self.records.iter().enumerate() {
            if record.parent_hash != expected_parent {
                return Err(SystemHalt::new(
                    FailureAxis::ExternalInjectionDetected,
                    &format!("Evidence chain broken at record {}: parent hash mismatch", i),
                ));
            }

            let recomputed_hash = record.compute_hash();
            if recomputed_hash != record.record_hash {
                return Err(SystemHalt::new(
                    FailureAxis::ExternalInjectionDetected,
                    &format!("Evidence record {} tampered: hash mismatch", i),
                ));
            }

            expected_parent = record.record_hash.clone();
        }

        Ok(())
    }

    /// Get record count
    pub fn len(&self) -> u64 {
        self.total_records
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    /// Get record at index
    pub fn get(&self, index: u64) -> Option<&AuditRecord> {
        if index >= self.total_records {
            return None;
        }
        self.records.get(index as usize)
    }

    /// Iterate over records
    pub fn iter(&self) -> impl Iterator<Item = &AuditRecord> {
        self.records.iter()
    }

    /// Generate compliance summary
    pub fn compliance_summary(&self) -> ComplianceSummary {
        let mut summary = ComplianceSummary::default();

        for record in &self.records {
            match record.failure_class.as_str() {
                "TIMING_OK" => summary.timing_ok_count += 1,
                "TIMING_DRIFT" => summary.timing_drift_count += 1,
                "DATA_CORRUPTION" => summary.data_corruption_count += 1,
                "INJECTION_DETECTED" => summary.injection_detected_count += 1,
                "AUTHORITY_INVERSION" => summary.authority_inversion_count += 1,
                _ => {}
            }
        }

        summary.total_records = self.total_records;
        summary.fingerprint = hex::encode(self.compute_fingerprint());

        summary
    }

    /// Export to JSON for external audit
    pub fn to_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Export to CSV timeline
    pub fn to_csv_timeline(&self) -> String {
        let mut csv = String::from("sequence,timestamp_ns,failure_class,evidence\n");

        for record in &self.records {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                record.sequence,
                record.timestamp,
                record.failure_class,
                record.evidence.replace("\"", "\\\"")
            ));
        }

        csv
    }
}

impl Default for EvidenceRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Compliance summary for ISE audit
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_records: u64,
    pub timing_ok_count: u64,
    pub timing_drift_count: u64,
    pub data_corruption_count: u64,
    pub injection_detected_count: u64,
    pub authority_inversion_count: u64,
    pub fingerprint: String,
}

impl ComplianceSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        (self.timing_ok_count as f64) / (self.total_records as f64)
    }

    pub fn total_failures(&self) -> u64 {
        self.timing_drift_count
            + self.data_corruption_count
            + self.injection_detected_count
            + self.authority_inversion_count
    }

    pub fn is_compliant(&self) -> bool {
        self.total_failures() == 0 && self.total_records > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_repository_append() {
        let mut repo = EvidenceRepository::new();

        repo.append_evidence(1000000, EvidenceClass::TimingOk, "Normal".to_string());
        repo.append_evidence(2000000, EvidenceClass::TimingOk, "Normal".to_string());

        assert_eq!(repo.len(), 2);
    }

    #[test]
    fn test_chain_integrity() {
        let mut repo = EvidenceRepository::new();

        repo.append_evidence(1000000, EvidenceClass::TimingOk, "Normal".to_string());
        repo.append_evidence(2000000, EvidenceClass::TimingDrift, "Drift detected".to_string());

        assert!(repo.verify_chain_integrity().is_ok());
    }

    #[test]
    fn test_fingerprint_determinism() {
        let mut repo1 = EvidenceRepository::new();
        let mut repo2 = EvidenceRepository::new();

        for i in 0..10 {
            repo1.append_evidence(
                (i * 1000000) as u128,
                EvidenceClass::TimingOk,
                "Normal".to_string(),
            );
            repo2.append_evidence(
                (i * 1000000) as u128,
                EvidenceClass::TimingOk,
                "Normal".to_string(),
            );
        }

        let fp1 = repo1.compute_fingerprint();
        let fp2 = repo2.compute_fingerprint();

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_compliance_summary() {
        let mut repo = EvidenceRepository::new();

        for _ in 0..8 {
            repo.append_evidence(
                0,
                EvidenceClass::TimingOk,
                "OK".to_string(),
            );
        }

        repo.append_evidence(0, EvidenceClass::TimingDrift, "Drift".to_string());
        repo.append_evidence(0, EvidenceClass::DataCorruption, "Corruption".to_string());

        let summary = repo.compliance_summary();
        assert_eq!(summary.total_records, 10);
        assert_eq!(summary.timing_ok_count, 8);
        assert_eq!(summary.timing_drift_count, 1);
        assert_eq!(summary.data_corruption_count, 1);
        assert!(!summary.is_compliant());
    }
}
