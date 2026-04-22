// Copyright (c) 2026 OBINNA JAMES EJIOFOR
// All Rights Reserved.
//
// This file is part of the M.V.R.ESPRINT1 Sovereign Execution System.

#![deny(unsafe_code)]

use crate::failure_axis::{FailureAxis, SystemHalt};
use crate::setpoint_guard::Setpoint;

/// Deployment authority stages used to gate closed-loop behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentPhase {
    Phase0Passive,
    Phase1Advisory,
    Phase2Guardrail,
    Phase3AssistedControl,
}

impl DeploymentPhase {
    pub fn from_env_value(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "0" | "phase0" | "phase_0" | "passive" => Some(Self::Phase0Passive),
            "1" | "phase1" | "phase_1" | "advisory" => Some(Self::Phase1Advisory),
            "2" | "phase2" | "phase_2" | "guardrail" => Some(Self::Phase2Guardrail),
            "3" | "phase3" | "phase_3" | "assisted" | "assisted_control" => {
                Some(Self::Phase3AssistedControl)
            }
            _ => None,
        }
    }
}

/// Narrow scope constraints for Phase 3 assisted-control operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssistedControlScope {
    pub max_abs_p_mw: f64,
    pub max_abs_q_mvar: f64,
    pub require_operator_ack: bool,
}

impl Default for AssistedControlScope {
    fn default() -> Self {
        Self {
            max_abs_p_mw: 25.0,
            max_abs_q_mvar: 10.0,
            require_operator_ack: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseControlGate {
    pub phase: DeploymentPhase,
    pub scope: AssistedControlScope,
}

impl Default for PhaseControlGate {
    fn default() -> Self {
        Self {
            phase: DeploymentPhase::Phase2Guardrail,
            scope: AssistedControlScope::default(),
        }
    }
}

impl PhaseControlGate {
    pub fn from_env() -> Result<Self, SystemHalt> {
        let phase = std::env::var("MVRE_DEPLOYMENT_PHASE")
            .ok()
            .map(|v| {
                DeploymentPhase::from_env_value(&v).ok_or_else(|| {
                    SystemHalt::new(
                        FailureAxis::UnauthorizedMode,
                        "Invalid MVRE_DEPLOYMENT_PHASE value",
                    )
                })
            })
            .transpose()?
            .unwrap_or(DeploymentPhase::Phase2Guardrail);

        Ok(Self {
            phase,
            scope: AssistedControlScope::default(),
        })
    }

    pub fn ensure_assisted_control_authorized(
        &self,
        operator_ack_token: Option<&str>,
    ) -> Result<(), SystemHalt> {
        if self.phase != DeploymentPhase::Phase3AssistedControl {
            return Err(SystemHalt::new(
                FailureAxis::AuthorityInversionAttempt,
                "Closed-loop authority is only permitted in Phase 3",
            ));
        }

        if self.scope.require_operator_ack {
            match operator_ack_token {
                Some(token) if !token.trim().is_empty() => Ok(()),
                _ => Err(SystemHalt::new(
                    FailureAxis::AuthorityInversionAttempt,
                    "Phase 3 requires a non-empty operator acknowledgment token",
                )),
            }
        } else {
            Ok(())
        }
    }

    pub fn clamp_to_assisted_scope(&self, desired: Setpoint) -> Setpoint {
        let p = desired
            .p
            .clamp(-self.scope.max_abs_p_mw, self.scope.max_abs_p_mw);
        let q = desired
            .q
            .clamp(-self.scope.max_abs_q_mvar, self.scope.max_abs_q_mvar);
        Setpoint { p, q, ..desired }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_three_requires_operator_ack() {
        let gate = PhaseControlGate {
            phase: DeploymentPhase::Phase3AssistedControl,
            scope: AssistedControlScope {
                require_operator_ack: true,
                ..AssistedControlScope::default()
            },
        };

        let denied = gate.ensure_assisted_control_authorized(None);
        assert!(denied.is_err());
        assert!(gate
            .ensure_assisted_control_authorized(Some("ops-ack-2026-04-17"))
            .is_ok());
    }

    #[test]
    fn non_phase_three_is_denied() {
        let gate = PhaseControlGate::default();
        let denied = gate.ensure_assisted_control_authorized(Some("ack"));
        assert!(denied.is_err());
    }

    #[test]
    fn phase_three_scope_clamps_requested_setpoint() {
        let gate = PhaseControlGate {
            phase: DeploymentPhase::Phase3AssistedControl,
            scope: AssistedControlScope {
                max_abs_p_mw: 10.0,
                max_abs_q_mvar: 2.0,
                require_operator_ack: false,
            },
        };

        let desired = Setpoint {
            p: 18.0,
            q: -6.0,
            ts: 123,
        };
        let clamped = gate.clamp_to_assisted_scope(desired);
        assert_eq!(
            clamped,
            Setpoint {
                p: 10.0,
                q: -2.0,
                ts: 123,
            }
        );
    }

    #[test]
    fn env_phase_parser_accepts_assisted_alias() {
        assert_eq!(
            DeploymentPhase::from_env_value("assisted_control"),
            Some(DeploymentPhase::Phase3AssistedControl)
        );
    }
}
