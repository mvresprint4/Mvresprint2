// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

#![deny(unsafe_code)]

use crate::failure_axis::SystemHalt;
use crate::telemetry::{Disturbance, TelemetryFrame};
use crate::tlbss_types::{BinaryState, ResonanceCurve};
use crate::sovereign_trace::{SovereignTrace, SovereignTraceLog};
use sha2::{Digest, Sha256};

/// Discrete substrate state.
#[derive(Debug, Clone)]
pub struct SubstrateState {
    pub symbolic: BinaryState,
    pub cognitive: BinaryState,
    pub biological: BinaryState,
    pub resonance: ResonanceCurve,
}

/// Belief state for epistemic mode selection.
#[derive(Debug, Clone)]
pub struct BeliefState {
    pub confidence: f64,
    pub covariance: [[f64; 3]; 3],
    pub observability_score: f64,
}

/// Mode selector for the MVRE epistemic controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    Bayesian,
    Robust,
    Viability,
    Safe,
}

/// Canonical kernel state object.
#[derive(Debug, Clone)]
pub struct KernelState {
    pub substrate: SubstrateState,
    pub belief: BeliefState,
    pub mode: ControlMode,
    pub timestamp: u64,
    pub last_control_signal: ControlSignal,
    pub hardware_actuation_permitted: bool,
    pub last_validation_passed: bool,
    pub last_fingerprint: [u8; 32],
}

/// Control commands issued by the MVRE kernel.
#[derive(Debug, Clone, Default)]
pub struct ControlSignal {
    pub halt: bool,
    pub shed_load: bool,
    pub lock_state: bool,
    pub drive_command: Option<u8>,
}

impl KernelState {
    pub fn new(substrate: SubstrateState, belief: BeliefState, mode: ControlMode, timestamp: u64) -> Self {
        Self {
            substrate,
            belief,
            mode,
            timestamp,
            last_control_signal: ControlSignal::default(),
            hardware_actuation_permitted: true,
            last_validation_passed: true,
            last_fingerprint: [0u8; 32],
        }
    }

    fn mode_id(mode: ControlMode) -> u8 {
        match mode {
            ControlMode::Bayesian => 0,
            ControlMode::Robust => 1,
            ControlMode::Viability => 2,
            ControlMode::Safe => 3,
        }
    }

    pub fn compute_execution_fingerprint(
        &self,
        control: &ControlSignal,
        disturbance: &Disturbance,
        next_substrate: &SubstrateState,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        hasher.update(self.substrate.symbolic.as_u8().to_le_bytes());
        hasher.update(self.substrate.cognitive.as_u8().to_le_bytes());
        hasher.update(self.substrate.biological.as_u8().to_le_bytes());
        hasher.update(self.substrate.resonance.frequency_hz.to_le_bytes());
        hasher.update(self.substrate.resonance.amplitude.to_le_bytes());
        hasher.update(self.substrate.resonance.damping_factor.to_le_bytes());

        hasher.update([control.halt as u8]);
        hasher.update([control.shed_load as u8]);
        hasher.update([control.lock_state as u8]);
        hasher.update([control.drive_command.unwrap_or(255)]);

        hasher.update([disturbance.symbolic]);
        hasher.update([disturbance.cognitive]);
        hasher.update([disturbance.biological]);
        hasher.update(disturbance.energy_norm.to_le_bytes());

        hasher.update(next_substrate.symbolic.as_u8().to_le_bytes());
        hasher.update(next_substrate.cognitive.as_u8().to_le_bytes());
        hasher.update(next_substrate.biological.as_u8().to_le_bytes());
        hasher.update(next_substrate.resonance.frequency_hz.to_le_bytes());
        hasher.update(next_substrate.resonance.amplitude.to_le_bytes());
        hasher.update(next_substrate.resonance.damping_factor.to_le_bytes());

        hasher.update(self.belief.confidence.to_le_bytes());
        for row in &self.belief.covariance {
            for value in row {
                hasher.update(value.to_le_bytes());
            }
        }
        hasher.update(self.belief.observability_score.to_le_bytes());

        hasher.update([Self::mode_id(self.mode)]);
        hasher.update(self.timestamp.to_le_bytes());

        hasher.finalize().into()
    }

    pub fn validate_actuation(&self, control: &ControlSignal, telemetry: &TelemetryFrame) -> bool {
        let observability = calculate_observability(&self.substrate, telemetry);
        let intended_command_present = control.halt || control.shed_load || control.lock_state || control.drive_command.is_some();

        telemetry.observed_output.is_finite()
            && telemetry.disturbance.energy_norm >= 0.0
            && observability >= 0.0
            && intended_command_present
    }
}

/// Topology consistency score feeding identifiability.
pub fn topology_consistency_score(state: &SubstrateState) -> f64 {
    let score = 1.0 - ((state.symbolic.as_u8() as f64
        + state.cognitive.as_u8() as f64
        + state.biological.as_u8() as f64)
        / 3.0);
    score.max(0.0)
}

/// The identifiability functional I(b, s).
pub fn identifiability_metric(belief: &BeliefState, state: &SubstrateState) -> f64 {
    let fisher = belief.covariance[0][0].abs() + belief.covariance[1][1].abs() + belief.covariance[2][2].abs();
    let topology = topology_consistency_score(state);
    fisher * topology
}

/// Bayesian update operator for belief.
pub fn bayesian_update(confidence: f64, telemetry: &TelemetryFrame) -> f64 {
    let delta = telemetry.observed_output.abs() * 1e-3;
    (confidence + delta).min(1.0)
}

/// Covariance update operator for belief.
pub fn update_covariance(covariance: [[f64; 3]; 3], telemetry: &TelemetryFrame) -> [[f64; 3]; 3] {
    let mut next = covariance;
    let noise = telemetry.observed_output.abs() * 1e-4;
    for i in 0..3 {
        next[i][i] = (next[i][i] + noise).min(1.0);
    }
    next
}

/// Observability score from substrate and telemetry.
pub fn calculate_observability(state: &SubstrateState, telemetry: &TelemetryFrame) -> f64 {
    let base = state.symbolic.as_u8() as f64 + state.cognitive.as_u8() as f64 + state.biological.as_u8() as f64;
    let noise = telemetry.observed_output.abs() * 1e-2;
    (base / 3.0).max(0.0) * (1.0 - noise)
}

impl BeliefState {
    pub fn update(&self, telemetry: &TelemetryFrame, substrate: &SubstrateState) -> Self {
        let confidence = bayesian_update(self.confidence, telemetry);
        let covariance = update_covariance(self.covariance, telemetry);
        let observability_score = calculate_observability(substrate, telemetry);

        Self {
            confidence,
            covariance,
            observability_score,
        }
    }
}

impl SubstrateState {
    pub fn transition(&self, control: &ControlSignal, disturbance: &Disturbance) -> Self {
        let symbolic = if control.halt {
            self.symbolic
        } else {
            BinaryState::from_u8((self.symbolic.as_u8() ^ disturbance.symbolic) & 1).unwrap_or(self.symbolic)
        };

        let cognitive = if control.shed_load {
            self.cognitive
        } else {
            BinaryState::from_u8((self.cognitive.as_u8() ^ disturbance.cognitive) & 1).unwrap_or(self.cognitive)
        };

        let biological = if control.lock_state {
            self.biological
        } else {
            BinaryState::from_u8((self.biological.as_u8() ^ disturbance.biological) & 1).unwrap_or(self.biological)
        };

        let resonance = ResonanceCurve {
            frequency_hz: self.resonance.frequency_hz,
            amplitude: self.resonance.amplitude,
            damping_factor: self.resonance.damping_factor + disturbance.energy_norm as f32 * 0.01,
        };

        SubstrateState {
            symbolic,
            cognitive,
            biological,
            resonance,
        }
    }
}

/// Mode selection law sigma(b, s).
pub fn select_mode(belief: &BeliefState, substrate: &SubstrateState) -> ControlMode {
    let i = identifiability_metric(belief, substrate);
    if i > 0.85 {
        ControlMode::Bayesian
    } else if i > 0.50 {
        ControlMode::Robust
    } else if substrate.symbolic == BinaryState::Zero || substrate.cognitive == BinaryState::Zero || substrate.biological == BinaryState::Zero {
        ControlMode::Viability
    } else {
        ControlMode::Safe
    }
}

/// Policy dispatcher.
pub fn compute_control(mode: ControlMode, state: &SubstrateState, belief: &BeliefState) -> ControlSignal {
    match mode {
        ControlMode::Bayesian => bayesian_controller(state, belief),
        ControlMode::Robust => robust_controller(state),
        ControlMode::Viability => viability_controller(state),
        ControlMode::Safe => isolation_controller(state),
    }
}

fn bayesian_controller(_state: &SubstrateState, belief: &BeliefState) -> ControlSignal {
    ControlSignal {
        halt: false,
        shed_load: belief.confidence < 0.2,
        lock_state: false,
        drive_command: Some(1),
    }
}

fn robust_controller(_state: &SubstrateState) -> ControlSignal {
    ControlSignal {
        halt: false,
        shed_load: true,
        lock_state: false,
        drive_command: Some(0),
    }
}

fn viability_controller(state: &SubstrateState) -> ControlSignal {
    let candidate = robust_controller(state);
    if admissible(&candidate, state) {
        candidate
    } else {
        nearest_admissible(state)
    }
}

fn isolation_controller(_state: &SubstrateState) -> ControlSignal {
    ControlSignal {
        halt: true,
        shed_load: true,
        lock_state: true,
        drive_command: None,
    }
}

fn admissible(candidate: &ControlSignal, _state: &SubstrateState) -> bool {
    !candidate.halt
}

fn nearest_admissible(_state: &SubstrateState) -> ControlSignal {
    ControlSignal {
        halt: false,
        shed_load: true,
        lock_state: true,
        drive_command: Some(0),
    }
}

/// Hybrid kernel execution loop: z_{t+1} = K(z_t).
pub fn execute_cycle(kernel: &mut KernelState, telemetry: TelemetryFrame, trace_log: &mut SovereignTraceLog) {
    kernel.belief = kernel.belief.update(&telemetry, &kernel.substrate);
    kernel.mode = select_mode(&kernel.belief, &kernel.substrate);
    let control = compute_control(kernel.mode, &kernel.substrate, &kernel.belief);
    let next_substrate = kernel.substrate.transition(&control, &telemetry.disturbance);
    let fingerprint = kernel.compute_execution_fingerprint(&control, &telemetry.disturbance, &next_substrate);
    let validation_passed = kernel.validate_actuation(&control, &telemetry);
    kernel.hardware_actuation_permitted = validation_passed && !control.halt;
    kernel.last_control_signal = control.clone();
    kernel.last_validation_passed = validation_passed;
    kernel.last_fingerprint = fingerprint;

    if kernel.hardware_actuation_permitted {
        kernel.substrate = next_substrate;
    }

    kernel.timestamp += 1;

    let actual_output = if kernel.hardware_actuation_permitted { 1.0 } else { 0.0 };
    let mut trace = SovereignTrace::new(
        kernel.timestamp,
        telemetry.observed_output,
        actual_output,
        crate::regulatory_policy::GovernanceMode::Normal,
        crate::regulatory_policy::LegalCitation::default(),
    );
    trace.execution_fingerprint = Some(fingerprint.to_vec());
    trace_log.append_state_transition(trace);
}

/// Deterministic kernel trace record for replay.
pub fn commit_trace(_trace: SovereignTrace) -> Result<(), SystemHalt> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::TelemetryFrame;
    use crate::tlbss_types::BinaryState;
    use crate::sovereign_trace::SovereignTraceLog;

    fn base_kernel_state() -> KernelState {
        KernelState::new(
            SubstrateState {
                symbolic: BinaryState::One,
                cognitive: BinaryState::One,
                biological: BinaryState::One,
                resonance: ResonanceCurve {
                    frequency_hz: 60.0,
                    amplitude: 0.1,
                    damping_factor: 0.05,
                },
            },
            BeliefState {
                confidence: 0.0,
                covariance: [[0.0; 3]; 3],
                observability_score: 0.0,
            },
            ControlMode::Safe,
            0,
        )
    }

    #[test]
    fn test_compute_execution_fingerprint_is_deterministic() {
        let kernel = base_kernel_state();
        let disturbance = Disturbance::new(1, 0, 1, 0.5).unwrap();
        let control = ControlSignal {
            halt: true,
            shed_load: true,
            lock_state: true,
            drive_command: None,
        };
        let next_substrate = kernel.substrate.transition(&control, &disturbance);

        let fp1 = kernel.compute_execution_fingerprint(&control, &disturbance, &next_substrate);
        let fp2 = kernel.compute_execution_fingerprint(&control, &disturbance, &next_substrate);

        assert_eq!(fp1, fp2, "The execution fingerprint must be deterministic");
        assert_eq!(fp1.len(), 32);
    }

    #[test]
    fn test_execute_cycle_enforces_fail_closed_on_halt() {
        let mut kernel = base_kernel_state();
        let disturbance = Disturbance::new(0, 1, 0, 0.1).unwrap();
        let telemetry = TelemetryFrame::new(disturbance.clone(), 0.5, vec![]);
        let mut trace_log = SovereignTraceLog::new();

        let pre_state = kernel.substrate.clone();

        execute_cycle(&mut kernel, telemetry, &mut trace_log);

        assert!(!kernel.hardware_actuation_permitted, "Hardware actuation must be disabled when Safe mode issues a halt signal");
        assert_eq!(kernel.substrate.symbolic, pre_state.symbolic, "Substrate state must not advance when actuation is blocked");
        assert_eq!(kernel.substrate.cognitive, pre_state.cognitive);
        assert_eq!(kernel.substrate.biological, pre_state.biological);
    }
}
