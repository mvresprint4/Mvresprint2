// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// PTP Compliance Validation Module
// IEEE 1588-2008 conformance checking and jitter tolerance verification.
//
// PHASE 3 ENHANCEMENT:
// - Formal PTP compliance checking against IEEE 1588 Annex D
// - Jitter tolerance profile validation
// - HALT_0xABF3 threshold enforcement
// - Stratum and clock class tracking

#![deny(unsafe_code)]

use crate::canonical_time::{CanonicalTime, ptp_constants};
use crate::failure_axis::{FailureAxis, SystemHalt};
use serde::{Deserialize, Serialize};

/// IEEE 1588 Clock Class enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockClass {
    /// Master clock, traceable to PRC/atomic standard
    MasterTraceable = 6,
    /// Grandmaster clock
    Grandmaster = 7,
    /// Ordinary clock in locked state
    OrdinaryLocked = 13,
    /// Ordinary clock, not locked
    OrdinaryUnlocked = 187,
}

/// IEEE 1588 Stratum Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stratum {
    Prc = 1,      // Primary Reference Clock
    Sec = 2,      // Secondary Reference
    Ter = 3,      // Tertiary
    Local = 4,    // Local Stratum
    Gps = 5,      // GPS/GNSS traceable
}

/// PTP Jitter Tolerance Profile (per IEEE 1588 Annex D)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitterToleranceProfile {
    /// Maximum allowable input phase error (nanoseconds)
    pub max_phase_error_ns: i64,
    
    /// Maximum holdover drift rate (PPM)
    pub max_holdover_drift_ppm: i32,
    
    /// Maximum one-way delay (microseconds)
    pub max_owd_us: u64,
    
    /// Maximum clock skew (ns/s)
    pub max_clock_skew_ns_per_s: i64,
    
    /// Typical sync interval (seconds)
    pub sync_interval_sec: u32,
}

impl Default for JitterToleranceProfile {
    fn default() -> Self {
        // IEEE 1588 Annex D: Ordinary Clock (Grandmaster-capable)
        Self {
            max_phase_error_ns: ptp_constants::MAX_PHASE_OFFSET_NS,
            max_holdover_drift_ppm: ptp_constants::MAX_FREQUENCY_DRIFT_PPM,
            max_owd_us: ptp_constants::MAX_OWD_US,
            max_clock_skew_ns_per_s: 1,
            sync_interval_sec: 8,
        }
    }
}

/// PTP Compliance Validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtpCompliance {
    pub clock_class: ClockClass,
    pub stratum: Stratum,
    pub jitter_profile: JitterToleranceProfile,
    pub missed_syncs: u32,
    pub max_missed_syncs_allowed: u32,
}

impl PtpCompliance {
    pub fn new(clock_class: ClockClass, stratum: Stratum) -> Self {
        Self {
            clock_class,
            stratum,
            jitter_profile: JitterToleranceProfile::default(),
            missed_syncs: 0,
            max_missed_syncs_allowed: ptp_constants::MAX_MISSED_SYNCS,
        }
    }

    /// Validate phase offset against profile
    pub fn validate_phase_offset(&self, offset_ns: i128) -> Result<(), SystemHalt> {
        if offset_ns.abs() > (self.jitter_profile.max_phase_error_ns as i128) {
            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "Phase offset {} ns exceeds tolerance {} ns (HALT_0xABF3)",
                    offset_ns, self.jitter_profile.max_phase_error_ns
                ),
            ));
        }
        Ok(())
    }

    /// Validate frequency drift against profile
    pub fn validate_frequency_drift(&self, drift_ppm: i32) -> Result<(), SystemHalt> {
        if drift_ppm.abs() > self.jitter_profile.max_holdover_drift_ppm {
            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "Frequency drift {} ppm exceeds tolerance ±{} ppm (HALT_0xABF3)",
                    drift_ppm, self.jitter_profile.max_holdover_drift_ppm
                ),
            ));
        }
        Ok(())
    }

    /// Validate one-way delay
    pub fn validate_owd(&self, owd_us: u64) -> Result<(), SystemHalt> {
        if owd_us > self.jitter_profile.max_owd_us {
            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "One-way delay {} µs exceeds tolerance {} µs",
                    owd_us, self.jitter_profile.max_owd_us
                ),
            ));
        }
        Ok(())
    }

    /// Record missed sync packet
    pub fn record_missed_sync(&mut self) -> Result<(), SystemHalt> {
        self.missed_syncs = self.missed_syncs.saturating_add(1);
        
        if self.missed_syncs > self.max_missed_syncs_allowed {
            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "Missed syncs {} exceeds tolerance {} (HALT_0xABF3)",
                    self.missed_syncs, self.max_missed_syncs_allowed
                ),
            ));
        }
        Ok(())
    }

    /// Reset missed sync counter (sync packet received)
    pub fn reset_missed_sync(&mut self) {
        self.missed_syncs = 0;
    }

    /// Comprehensive compliance check
    pub fn validate_all(
        &self,
        phase_offset_ns: i128,
        frequency_drift_ppm: i32,
        owd_us: u64,
    ) -> Result<(), SystemHalt> {
        self.validate_phase_offset(phase_offset_ns)?;
        self.validate_frequency_drift(frequency_drift_ppm)?;
        self.validate_owd(owd_us)?;
        Ok(())
    }

    /// Get stratum chain string (for diagnostic output)
    pub fn stratum_chain(&self) -> String {
        format!(
            "Stratum {:?} @ ClockClass {:?}",
            self.stratum, self.clock_class
        )
    }
}

/// Jitter statistics for quality assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JitterStats {
    pub jitter_min_ns: i64,
    pub jitter_max_ns: i64,
    pub jitter_mean_ns: f64,
    pub jitter_stddev_ns: f64,
    pub samples: u64,
}

impl JitterStats {
    pub fn update(&mut self, jitter_ns: i64) {
        if self.samples == 0 {
            self.jitter_min_ns = jitter_ns;
            self.jitter_max_ns = jitter_ns;
            self.jitter_mean_ns = jitter_ns as f64;
            self.jitter_stddev_ns = 0.0;
        } else {
            let prev_mean = self.jitter_mean_ns;
            let new_count = self.samples as f64 + 1.0;
            self.jitter_mean_ns = prev_mean + (jitter_ns as f64 - prev_mean) / new_count;
            
            let variance_delta = (jitter_ns as f64 - prev_mean) * (jitter_ns as f64 - self.jitter_mean_ns);
            if self.samples > 0 {
                self.jitter_stddev_ns = (self.jitter_stddev_ns.powi(2) * (self.samples as f64 - 1.0) + variance_delta) / (self.samples as f64);
                self.jitter_stddev_ns = self.jitter_stddev_ns.sqrt();
            }
            
            self.jitter_min_ns = self.jitter_min_ns.min(jitter_ns);
            self.jitter_max_ns = self.jitter_max_ns.max(jitter_ns);
        }
        self.samples += 1;
    }

    /// Check if jitter profile is within tolerance
    pub fn exceeds_tolerance(&self, threshold_ns: i64) -> bool {
        self.jitter_max_ns > threshold_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptp_compliance_basic() {
        let compliance = PtpCompliance::new(ClockClass::OrdinaryLocked, Stratum::Local);
        
        // Phase within tolerance
        assert!(compliance.validate_phase_offset(100).is_ok());
        
        // Phase exceeding tolerance
        assert!(compliance.validate_phase_offset(500).is_err());
    }

    #[test]
    fn test_missed_sync_tracking() {
        let mut compliance = PtpCompliance::new(ClockClass::OrdinaryLocked, Stratum::Local);
        
        for _ in 0..10 {
            let _ = compliance.record_missed_sync();
        }
        
        assert_eq!(compliance.missed_syncs, 10);
        
        // 11th miss should trigger halt
        assert!(compliance.record_missed_sync().is_err());
    }

    #[test]
    fn test_jitter_statistics() {
        let mut stats = JitterStats::default();
        
        stats.update(10);
        stats.update(20);
        stats.update(30);
        
        assert_eq!(stats.samples, 3);
        assert_eq!(stats.jitter_min_ns, 10);
        assert_eq!(stats.jitter_max_ns, 30);
    }
}
