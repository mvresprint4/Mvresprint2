// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// ISE (Integration Simulation Environment) Harness
// Deterministic sandbox for PTP drift injection, parity fault simulation,
// and failure classification validation per IEEE 1588 compliance.
//
// PHASE 2 IMPLEMENTATION:
// - Controlled sandbox for clock drift and parity-check fault injection
// - Three execution modes: realtime, accelerated, step
// - Snapshot replay with deterministic kernel isolation
// - Immutable evidence repository for timing vs. data corruption classification

#![deny(unsafe_code)]

use crate::canonical_time::{CanonicalTime, ptp_constants};
use crate::failure_axis::{FailureAxis, SystemHalt};
use crate::deterministic_core::DetTime;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// ISE Execution Mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Real-time: Adheres to wall clock
    Realtime,
    /// Accelerated: Time runs N times faster (e.g., 60x)
    Accelerated(u32),
    /// Step: Manual time advancement for controlled injection
    Step,
}

/// Failure Classification for ISE Evidence Repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureClassification {
    /// Timing within IEEE 1588 tolerances
    TimingOk {
        phase_offset_ns: i128,
        freq_offset_ppm: i32,
    },
    /// Phase offset exceeds tolerance (HALT_0xABF3)
    TimingDriftPhase {
        phase_offset_ns: i128,
        threshold_ns: i64,
    },
    /// Frequency drift exceeds tolerance (HALT_0xABF3)
    TimingDriftFrequency {
        freq_offset_ppm: i32,
        threshold_ppm: i32,
    },
    /// Data corruption: Parity or hash mismatch
    DataCorruption {
        field: String,
        detected_via: String,  // "parity", "hash", "crc"
    },
    /// Adversarial injection detected
    InjectionDetected {
        injection_type: String,
        evidence: String,
    },
    /// Authority inversion or replay attack
    AuthorityInversion {
        reason: String,
    },
}

impl FailureClassification {
    /// Map to canonical FailureAxis for halt semantics
    pub fn to_failure_axis(&self) -> Option<FailureAxis> {
        match self {
            Self::TimingOk { .. } => None,
            Self::TimingDriftPhase { .. } => Some(FailureAxis::TimingDriftFailure),
            Self::TimingDriftFrequency { .. } => Some(FailureAxis::TimingDriftFailure),
            Self::DataCorruption { .. } => Some(FailureAxis::InternalInvariantBreach),
            Self::InjectionDetected { .. } => Some(FailureAxis::ExternalInjectionDetected),
            Self::AuthorityInversion { .. } => Some(FailureAxis::AuthorityInversionAttempt),
        }
    }
}

/// Single timing evidence record for ISE trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingEvidence {
    pub tick: u64,
    pub canonical_time_ns: u128,
    pub phase_offset_ns: i128,
    pub freq_offset_ppm: i32,
    pub jitter_ns: u64,
    pub classification: FailureClassification,
}

/// ISE Harness Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IseConfig {
    pub mode: ExecutionMode,
    pub max_ticks: u64,
    pub enable_clock_drift_injection: bool,
    pub enable_parity_fault_injection: bool,
    pub drift_injection_ppm: i32,      // Applied during drift window
    pub fault_injection_rate: f64,      // 0.0 to 1.0
}

impl Default for IseConfig {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Step,
            max_ticks: 1000,
            enable_clock_drift_injection: false,
            enable_parity_fault_injection: false,
            drift_injection_ppm: 0,
            fault_injection_rate: 0.0,
        }
    }
}

/// ISE Harness: Controlled sandbox for deterministic fault injection
#[derive(Debug)]
pub struct IseHarness {
    config: IseConfig,
    current_tick: u64,
    current_time_ns: u128,
    reference_time_ns: u128,
    evidence_log: VecDeque<TimingEvidence>,
    missed_syncs: u32,
    parity_error_count: u64,
    hash_error_count: u64,
}

impl IseHarness {
    /// Create new ISE harness with given configuration
    pub fn new(config: IseConfig) -> Self {
        let now_ns = (DetTime::canonical_now_ms().as_millis() as u128) * 1_000_000;
        
        Self {
            config,
            current_tick: 0,
            current_time_ns: now_ns,
            reference_time_ns: now_ns,
            evidence_log: VecDeque::new(),
            missed_syncs: 0,
            parity_error_count: 0,
            hash_error_count: 0,
        }
    }

    /// Advance simulation by one tick (for Step mode)
    pub fn step_tick(&mut self) -> Result<(), SystemHalt> {
        if self.current_tick >= self.config.max_ticks {
            return Err(SystemHalt::new(
                FailureAxis::InternalInvariantBreach,
                "ISE: Maximum tick count exceeded",
            ));
        }

        match self.config.mode {
            ExecutionMode::Step => {
                // Manual advancement: 1 ms per step
                self.current_time_ns += 1_000_000;
                self.reference_time_ns = self.reference_time_ns.saturating_add(1_000_000);
                self.advance_core()
            }
            ExecutionMode::Accelerated(factor) => {
                // Time runs `factor` times faster
                let delta_ns = (factor as u128) * 1_000_000;
                self.current_time_ns += delta_ns;
                self.reference_time_ns = self.reference_time_ns.saturating_add(delta_ns);
                self.advance_core()
            }
            ExecutionMode::Realtime => {
                // Wall-clock adherence
                let now_ns = (DetTime::canonical_now_ms().as_millis() as u128) * 1_000_000;
                if now_ns < self.current_time_ns {
                    // Not enough wall clock elapsed
                    return Ok(());
                }
                self.current_time_ns = now_ns;
                self.advance_core()
            }
        }
    }

    /// Core tick advancement: validation and classification
    fn advance_core(&mut self) -> Result<(), SystemHalt> {
        self.current_tick = self.current_tick.saturating_add(1);

        // Calculate phase offset from reference
        let phase_offset_ns = (self.current_time_ns as i128) - (self.reference_time_ns as i128);

        // Apply clock drift if injection enabled
        let freq_offset_ppm = if self.config.enable_clock_drift_injection {
            self.config.drift_injection_ppm
        } else {
            0
        };

        // Apply parity fault injection if enabled
        if self.config.enable_parity_fault_injection && self.should_inject_parity_error() {
            self.parity_error_count += 1;
            
            let evidence = TimingEvidence {
                tick: self.current_tick,
                canonical_time_ns: self.current_time_ns,
                phase_offset_ns,
                freq_offset_ppm,
                jitter_ns: 0,
                classification: FailureClassification::DataCorruption {
                    field: format!("witness_{}", self.current_tick % 32),
                    detected_via: "parity".to_string(),
                },
            };

            self.evidence_log.push_back(evidence);

            return Err(SystemHalt::new(
                FailureAxis::InternalInvariantBreach,
                &format!(
                    "ISE: Parity error detected at tick {} (injected)",
                    self.current_tick
                ),
            ));
        }

        // Check phase offset tolerance (HALT_0xABF3 trigger)
        if phase_offset_ns.abs() > (ptp_constants::MAX_PHASE_OFFSET_NS as i128) {
            let evidence = TimingEvidence {
                tick: self.current_tick,
                canonical_time_ns: self.current_time_ns,
                phase_offset_ns,
                freq_offset_ppm,
                jitter_ns: 0,
                classification: FailureClassification::TimingDriftPhase {
                    phase_offset_ns,
                    threshold_ns: ptp_constants::MAX_PHASE_OFFSET_NS,
                },
            };

            self.evidence_log.push_back(evidence);

            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "ISE: Phase offset {} ns exceeds tolerance (HALT_0xABF3)",
                    phase_offset_ns
                ),
            ));
        }

        // Check frequency drift tolerance (HALT_0xABF3 trigger)
        if freq_offset_ppm.abs() > ptp_constants::MAX_FREQUENCY_DRIFT_PPM {
            let evidence = TimingEvidence {
                tick: self.current_tick,
                canonical_time_ns: self.current_time_ns,
                phase_offset_ns,
                freq_offset_ppm,
                jitter_ns: 0,
                classification: FailureClassification::TimingDriftFrequency {
                    freq_offset_ppm,
                    threshold_ppm: ptp_constants::MAX_FREQUENCY_DRIFT_PPM,
                },
            };

            self.evidence_log.push_back(evidence);

            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "ISE: Frequency drift {} ppm exceeds tolerance (HALT_0xABF3)",
                    freq_offset_ppm
                ),
            ));
        }

        // Normal case: timing within tolerance
        let evidence = TimingEvidence {
            tick: self.current_tick,
            canonical_time_ns: self.current_time_ns,
            phase_offset_ns,
            freq_offset_ppm,
            jitter_ns: 0,
            classification: FailureClassification::TimingOk {
                phase_offset_ns,
                freq_offset_ppm,
            },
        };

        self.evidence_log.push_back(evidence);
        Ok(())
    }

    /// Probabilistic parity error injection
    fn should_inject_parity_error(&self) -> bool {
        if self.config.fault_injection_rate <= 0.0 {
            return false;
        }

        // Simple deterministic injection based on tick number
        let normalized = ((self.current_tick as f64 * self.config.fault_injection_rate) % 1.0).fract();
        normalized < self.config.fault_injection_rate
    }

    /// Get evidence log (immutable view)
    pub fn evidence_log(&self) -> &VecDeque<TimingEvidence> {
        &self.evidence_log
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get current canonical time
    pub fn current_time(&self) -> CanonicalTime {
        CanonicalTime::from_nanos(self.current_time_ns)
    }

    /// Get statistics summary
    pub fn statistics(&self) -> IseStatistics {
        let total_ticks = self.current_tick;
        let mut timing_ok_count = 0;
        let mut timing_drift_count = 0;
        let mut data_corruption_count = 0;
        let mut injection_detected_count = 0;
        let mut authority_inversion_count = 0;

        for evidence in &self.evidence_log {
            match &evidence.classification {
                FailureClassification::TimingOk { .. } => timing_ok_count += 1,
                FailureClassification::TimingDriftPhase { .. }
                | FailureClassification::TimingDriftFrequency { .. } => {
                    timing_drift_count += 1;
                }
                FailureClassification::DataCorruption { .. } => data_corruption_count += 1,
                FailureClassification::InjectionDetected { .. } => injection_detected_count += 1,
                FailureClassification::AuthorityInversion { .. } => authority_inversion_count += 1,
            }
        }

        IseStatistics {
            total_ticks,
            timing_ok_count,
            timing_drift_count,
            data_corruption_count,
            injection_detected_count,
            authority_inversion_count,
            parity_errors_injected: self.parity_error_count,
            hash_errors_injected: self.hash_error_count,
            missed_syncs: self.missed_syncs,
        }
    }

    /// Compute deterministic fingerprint of execution
    pub fn compute_fingerprint(&self) -> Vec<u8> {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();

        // Hash all evidence records
        for evidence in &self.evidence_log {
            hasher.update(evidence.tick.to_le_bytes());
            hasher.update(evidence.canonical_time_ns.to_le_bytes());
            hasher.update(evidence.phase_offset_ns.to_le_bytes());
            hasher.update(evidence.freq_offset_ppm.to_le_bytes());
        }

        hasher.finalize().to_vec()
    }
}

/// ISE Execution Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IseStatistics {
    pub total_ticks: u64,
    pub timing_ok_count: u64,
    pub timing_drift_count: u64,
    pub data_corruption_count: u64,
    pub injection_detected_count: u64,
    pub authority_inversion_count: u64,
    pub parity_errors_injected: u64,
    pub hash_errors_injected: u64,
    pub missed_syncs: u32,
}

impl IseStatistics {
    /// Compute success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_ticks == 0 {
            return 0.0;
        }
        (self.timing_ok_count as f64) / (self.total_ticks as f64)
    }

    /// Check if ISE execution meets compliance criteria
    pub fn is_compliant(&self) -> bool {
        // All ticks must be timing-ok or classified as expected injection
        let unclassified = self.total_ticks
            - self.timing_ok_count
            - self.timing_drift_count
            - self.data_corruption_count
            - self.injection_detected_count
            - self.authority_inversion_count;

        unclassified == 0 && self.missed_syncs <= ptp_constants::MAX_MISSED_SYNCS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ise_step_mode_basic() {
        let mut config = IseConfig::default();
        config.mode = ExecutionMode::Step;
        config.max_ticks = 10;

        let mut harness = IseHarness::new(config);

        for _ in 0..10 {
            let result = harness.step_tick();
            assert!(result.is_ok(), "Step mode should succeed");
        }

        let stats = harness.statistics();
        assert_eq!(stats.total_ticks, 10);
        assert_eq!(stats.timing_ok_count, 10);
    }

    #[test]
    fn test_ise_drift_injection() {
        let mut config = IseConfig::default();
        config.mode = ExecutionMode::Step;
        config.max_ticks = 5;
        config.enable_clock_drift_injection = true;
        config.drift_injection_ppm = 50;  // Exceed threshold

        let mut harness = IseHarness::new(config);

        let result = harness.step_tick();
        assert!(
            result.is_err(),
            "Should fail due to excessive drift injection"
        );

        let stats = harness.statistics();
        assert_eq!(stats.timing_drift_count, 1);
    }

    #[test]
    fn test_ise_fingerprint_determinism() {
        let config = IseConfig::default();
        let mut harness1 = IseHarness::new(config.clone());
        let mut harness2 = IseHarness::new(config);

        for _ in 0..5 {
            let _ = harness1.step_tick();
            let _ = harness2.step_tick();
        }

        let fp1 = harness1.compute_fingerprint();
        let fp2 = harness2.compute_fingerprint();

        assert_eq!(fp1, fp2, "Fingerprints must be identical for deterministic runs");
    }
}
