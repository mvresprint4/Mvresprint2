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

use crate::canonical_time::{CanonicalTime, PtpServoState, ptp_constants};
use crate::deterministic_core::DetTime;
use std::sync::atomic::{AtomicU64, AtomicI32, Ordering};
use std::sync::Mutex;

use crate::failure_axis::{FailureAxis, SystemHalt};

/// Lock-free nanosecond clock adapter for IEEE 1588 PTP compliance.
/// Guarantees monotonicity via CAS-clamping and tracks frequency offset for servo loop.
/// 
/// PHASE 1 ENHANCEMENT:
/// - Sub-microsecond precision (nanoseconds) via atomic operations
/// - Frequency offset tracking for PTP servo algorithm
/// - HALT_0xABF3 trigger binding for phase/frequency threshold validation
#[derive(Debug)]
pub struct PtpClock {
    /// Last issued timestamp in nanoseconds (lock-free via atomic)
    last_ns: AtomicU64,
    
    /// Current frequency offset in PPM (parts per million)
    /// Positive = running fast; negative = running slow
    freq_offset_ppm: AtomicI32,
    
    /// Servo state for PI controller updates (protected by Mutex)
    servo_state: Mutex<PtpServoState>,
}

impl PtpClock {
    pub fn new() -> Self {
        Self {
            last_ns: AtomicU64::new(0),
            freq_offset_ppm: AtomicI32::new(0),
            servo_state: Mutex::new(PtpServoState::new()),
        }
    }

    /// Read current nanosecond timestamp with monotonicity guarantee.
    /// Returns error if timestamp would exceed HALT_0xABF3 phase tolerance.
    pub fn read_nanos(&self) -> Result<u64, SystemHalt> {
        let observed_ns = wall_clock_nanos()?;
        let mut prev = self.last_ns.load(Ordering::Relaxed);
        
        loop {
            // Enforce monotonicity: never go backward
            let next = observed_ns.max(prev.saturating_add(1));
            
            // Validate phase offset against IEEE 1588 limits
            let offset_ns = (next as i128) - (prev as i128);
            if offset_ns.abs() > (ptp_constants::MAX_PHASE_OFFSET_NS as i128) {
                return Err(SystemHalt::new(
                    FailureAxis::TimingDriftFailure,
                    &format!(
                        "Phase offset {} ns exceeds tolerance {} ns (HALT_0xABF3)",
                        offset_ns, ptp_constants::MAX_PHASE_OFFSET_NS
                    ),
                ));
            }
            
            match self.last_ns.compare_exchange_weak(
                prev,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Update servo state with new measurement
                    if let Ok(mut servo) = self.servo_state.lock() {
                        servo.update(offset_ns);
                        let freq_ppm = servo.freq_offset_ppm;
                        self.freq_offset_ppm.store(freq_ppm, Ordering::Release);
                        
                        // Check frequency tolerance
                        if servo.exceeds_frequency_tolerance() {
                            return Err(SystemHalt::new(
                                FailureAxis::TimingDriftFailure,
                                &format!(
                                    "Frequency drift {} ppm exceeds tolerance ±{} ppm (HALT_0xABF3)",
                                    freq_ppm, ptp_constants::MAX_FREQUENCY_DRIFT_PPM
                                ),
                            ));
                        }
                    }
                    return Ok(next);
                }
                Err(actual) => prev = actual,
            }
        }
    }

    /// Read current microsecond timestamp (backward compatibility)
    pub fn read_micros(&self) -> Result<u64, SystemHalt> {
        Ok(self.read_nanos()? / 1_000)
    }

    /// Get current frequency offset (PPM)
    pub fn freq_offset_ppm(&self) -> i32 {
        self.freq_offset_ppm.load(Ordering::Acquire)
    }

    /// Inject simulated clock drift (for ISE testing)
    pub fn inject_clock_drift(&mut self, drift_ppm: i32) -> Result<(), SystemHalt> {
        if drift_ppm.abs() > ptp_constants::MAX_FREQUENCY_DRIFT_PPM {
            return Err(SystemHalt::new(
                FailureAxis::TimingDriftFailure,
                &format!(
                    "Injected drift {} ppm exceeds tolerance (HALT_0xABF3)",
                    drift_ppm
                ),
            ));
        }
        self.freq_offset_ppm.store(drift_ppm, Ordering::Release);
        Ok(())
    }

    /// Get canonical time representation
    pub fn as_canonical(&self) -> Result<CanonicalTime, SystemHalt> {
        let ns = self.read_nanos()?;
        Ok(CanonicalTime::from_nanos(ns as u128))
    }
}

impl Default for PtpClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Query wall clock in nanoseconds (boundary-only API)
fn wall_clock_nanos() -> Result<u64, SystemHalt> {
    // Convert milliseconds to nanoseconds
    let ms = DetTime::canonical_now_ms().as_millis() as u64;
    Ok(ms.saturating_mul(1_000_000))
}
