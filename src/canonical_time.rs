// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// CanonicalTime provides a deterministic timestamp representation for
// execution input and trace artifacts. It must be injected from the runtime
// gateway layer and never derived from host wall clock or instant APIs inside
// the core deterministic execution boundary.
//
// PHASE 1 ENHANCEMENT: Sub-microsecond precision (nanoseconds) for IEEE 1588 compliance.
// HALT_0xABF3 Trigger Integration: Phase offset and frequency drift thresholds formally bound.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// IEEE 1588 PTP Compliance Constants
/// Mapped to HALT_0xABF3 trigger thresholds per Annex D Jitter Tolerance Profiles
pub mod ptp_constants {
    /// Maximum allowable phase offset before HALT_0xABF3 fires (nanoseconds)
    /// IEEE 1588 typical: 250 ns (Annex D, Ordinary Clock)
    pub const MAX_PHASE_OFFSET_NS: i64 = 250;
    
    /// Maximum allowable frequency drift before HALT_0xABF3 fires (parts per million)
    /// ERCOT standard: ±5 ppm; ISE conservative: ±20 ppm
    pub const MAX_FREQUENCY_DRIFT_PPM: i32 = 20;
    
    /// Maximum one-way delay (microseconds)
    pub const MAX_OWD_US: u64 = 1000;  // 1 millisecond
    
    /// Maximum jitter peak before HALT_0xABF3 (nanoseconds)
    pub const MAX_JITTER_PEAK_NS: u64 = 100;
    
    /// Consecutive missed sync packets threshold before HALT
    pub const MAX_MISSED_SYNCS: u32 = 10;
}

/// Sub-microsecond canonical time: nanosecond precision
/// Replaces millisecond-only representation with full nanosecond range.
/// 
/// Internal representation: u128 nanoseconds (sufficient for ~10,000 years)
/// Canonical time MUST be injected from runtime boundary only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CanonicalTime {
    /// Timestamp in nanoseconds since UNIX epoch
    ns_since_epoch: u128,
}

impl CanonicalTime {
    /// Construct from milliseconds (for backward compatibility)
    pub fn from_millis(millis: u64) -> Self {
        Self {
            ns_since_epoch: (millis as u128) * 1_000_000,
        }
    }
    
    /// Construct from microseconds
    pub fn from_micros(micros: u64) -> Self {
        Self {
            ns_since_epoch: (micros as u128) * 1_000,
        }
    }
    
    /// Construct directly from nanoseconds
    pub fn from_nanos(nanos: u128) -> Self {
        Self {
            ns_since_epoch: nanos,
        }
    }
    
    /// Get as nanoseconds (highest precision)
    pub fn as_nanos(&self) -> u128 {
        self.ns_since_epoch
    }
    
    /// Get as microseconds (sub-millisecond precision)
    pub fn as_micros(&self) -> u64 {
        (self.ns_since_epoch / 1_000) as u64
    }
    
    /// Get as milliseconds (backward compatibility)
    pub fn as_millis(&self) -> u64 {
        (self.ns_since_epoch / 1_000_000) as u64
    }
    
    /// Compute phase offset in nanoseconds (signed difference)
    pub fn phase_offset_ns(&self, reference: CanonicalTime) -> i128 {
        (self.ns_since_epoch as i128) - (reference.ns_since_epoch as i128)
    }
    
    /// Check if phase offset exceeds IEEE 1588 limit
    pub fn exceeds_phase_tolerance(&self, reference: CanonicalTime) -> bool {
        let offset = self.phase_offset_ns(reference);
        offset.abs() > (ptp_constants::MAX_PHASE_OFFSET_NS as i128)
    }
}

impl std::fmt::Display for CanonicalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ns (μs: {}, ms: {})", 
            self.ns_since_epoch, 
            self.as_micros(), 
            self.as_millis())
    }
}

/// PTP Servo State: Tracks frequency offset and phase correction
/// Required for IEEE 1588 clock discipline (PI controller model)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PtpServoState {
    /// Current frequency offset (parts per million)
    /// Positive = running fast; negative = running slow
    pub freq_offset_ppm: i32,
    
    /// Accumulated phase error (nanoseconds)
    pub phase_error_ns: i128,
    
    /// Proportional gain (PI controller)
    pub kp: f64,
    
    /// Integral gain (PI controller)
    pub ki: f64,
}

impl PtpServoState {
    pub fn new() -> Self {
        Self {
            freq_offset_ppm: 0,
            phase_error_ns: 0,
            kp: 0.1,   // Typical value
            ki: 0.01,  // Typical value
        }
    }
    
    /// Update servo state based on new offset measurement
    pub fn update(&mut self, observed_offset_ns: i128) {
        // Proportional term
        let p_term = (self.kp * observed_offset_ns as f64) as i32;
        
        // Integral term
        self.phase_error_ns = self.phase_error_ns.saturating_add(observed_offset_ns);
        let i_term = (self.ki * self.phase_error_ns as f64) as i32;
        
        // Update frequency estimate
        self.freq_offset_ppm = (p_term + i_term).clamp(
            -ptp_constants::MAX_FREQUENCY_DRIFT_PPM,
            ptp_constants::MAX_FREQUENCY_DRIFT_PPM
        );
    }
    
    /// Check if frequency drift exceeds HALT_0xABF3 threshold
    pub fn exceeds_frequency_tolerance(&self) -> bool {
        self.freq_offset_ppm.abs() > ptp_constants::MAX_FREQUENCY_DRIFT_PPM
    }
}
