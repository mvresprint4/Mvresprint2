// Copyright © 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

#![deny(unsafe_code)]

use crate::failure_axis::SystemHalt;
use crate::telemetry::{Disturbance, TelemetryFrame};
use crate::tlbss_types::{BinaryState, ResonanceCurve};
use crate::sovereign_trace::{SovereignTrace, SovereignTraceLog};

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
}

/// Control commands issued by the MVRE kernel.
#[derive(Debug, Clone)]
pub struct ControlSignal {
    pub halt: bool,
    pub shed_load: bool,
    pub lock_state: bool,
    pub drive_command: Option<u8>,
}

impl ControlSignal {
    pub fn default() -> Self {
        Self {
            halt: false,
            shed_load: false,
            lock_state: false,
            drive_command: None,
        }
    }
}

impl KernelState {
    pub fn new(substrate: SubstrateState, belief: BeliefState, mode: ControlMode, timestamp: u64) -> Self {
        Self {
            substrate,
            belief,
            mode,
            timestamp,
        }
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
    kernel.substrate = kernel.substrate.transition(&control, &telemetry.disturbance);
    kernel.timestamp += 1;

    let trace = SovereignTrace::new(
        kernel.timestamp,
        telemetry.observed_output,
        if control.halt { 0.0 } else { 1.0 },
        crate::regulatory_policy::GovernanceMode::Normal,
        crate::regulatory_policy::LegalCitation::default(),
    );
    trace_log.append_state_transition(trace);
}

/// Deterministic kernel trace record for replay.
pub fn commit_trace(_trace: SovereignTrace) -> Result<(), SystemHalt> {
    Ok(())
}
