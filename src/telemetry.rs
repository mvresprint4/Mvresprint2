// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

#![deny(unsafe_code)]

use crate::failure_axis::SystemHalt;

/// Disturbance channels for the tri-lateral binary substrate.
#[derive(Debug, Clone)]
pub struct Disturbance {
    pub symbolic: u8,
    pub cognitive: u8,
    pub biological: u8,
    pub energy_norm: f64,
}

impl Disturbance {
    pub fn new(symbolic: u8, cognitive: u8, biological: u8, energy_norm: f64) -> Result<Self, SystemHalt> {
        if symbolic > 1 || cognitive > 1 || biological > 1 {
            return Err(SystemHalt::new(
                crate::failure_axis::FailureAxis::InternalInvariantBreach,
                "Disturbance channels must be binary",
            ));
        }

        Ok(Self {
            symbolic,
            cognitive,
            biological,
            energy_norm,
        })
    }
}

/// Structured telemetry arriving at the kernel for belief update.
#[derive(Debug, Clone)]
pub struct TelemetryFrame {
    pub disturbance: Disturbance,
    pub observed_output: f64,
    pub raw_payload: Vec<u8>,
}

impl TelemetryFrame {
    pub fn new(disturbance: Disturbance, observed_output: f64, raw_payload: Vec<u8>) -> Self {
        Self {
            disturbance,
            observed_output,
            raw_payload,
        }
    }
}
