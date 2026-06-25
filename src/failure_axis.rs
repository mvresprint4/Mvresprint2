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


/* Failure axis registry for deterministic halts */

use serde::{Deserialize, Serialize};

/// Canonical failure axis enumeration for deterministic halt classification.
/// Each axis corresponds to a specific class of system integrity violation.
///
/// HALT_0xABF3 BINDING:
/// TimingDriftFailure triggers HALT_0xABF3 when ANY of:
/// - Phase offset exceeds ±250 ns (IEEE 1588 Annex D tolerance)
/// - Frequency drift exceeds ±20 ppm (ERCOT standard)
/// - One-way delay variance exceeds bounds
/// - Missed sync packets exceed threshold (>10 consecutive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureAxis {
    /// Kernel invariant breach: Internal state corruption or logic violation
    InternalInvariantBreach,
    
    /// External injection detected: Adversarial fault or replay attack
    ExternalInjectionDetected,
    
    /// Timing drift failure: PTP synchronization loss or clock anomaly
    /// **HALT_0xABF3 Trigger:** Fires when phase/frequency thresholds exceeded
    TimingDriftFailure,
    
    /// Authority inversion attempt: Stratum degradation or permission violation
    AuthorityInversionAttempt,

    // Additional axes introduced for audit/hardening
    /// Reference timing lost or corrupted
    Reference,
    
    /// Feedback loop instability detected
    Feedback,
    
    /// Coupling violation between subsystems
    Coupling,
    
    /// Clock resolution inadequate for deterministic execution
    Resolution,
    
    /// Axiom 6/7 semantic misalignment (TLBSS geometry)
    Axiom6_7Misalignment,
    
    /// TPM unavailable or uninitialized
    TpmUnavailable,
    
    /// Unauthorized operational mode or capability invocation
    UnauthorizedMode,
}

/// System halt record: Contains failure axis and diagnostic message
#[derive(Debug, Clone)]
pub struct SystemHalt {
    pub axis: FailureAxis,
    pub message: String,
}

impl SystemHalt {
    pub fn new(axis: FailureAxis, message: &str) -> Self {
        Self {
            axis,
            message: message.to_string(),
        }
    }

    pub fn with_formatted(axis: FailureAxis, message: String) -> Self {
        Self { axis, message }
    }

    /// Map to HALT trigger code for operator display
    pub fn halt_code(&self) -> String {
        match self.axis {
            FailureAxis::TimingDriftFailure => "HALT_0xABF3".to_string(),
            FailureAxis::InternalInvariantBreach => "HALT_0xFEED".to_string(),
            FailureAxis::ExternalInjectionDetected => "HALT_0xBADF".to_string(),
            FailureAxis::AuthorityInversionAttempt => "HALT_0xDEAD".to_string(),
            FailureAxis::Reference => "HALT_0x0001".to_string(),
            FailureAxis::Feedback => "HALT_0x0002".to_string(),
            FailureAxis::Coupling => "HALT_0x0003".to_string(),
            FailureAxis::Resolution => "HALT_0x0004".to_string(),
            FailureAxis::Axiom6_7Misalignment => "HALT_0x0005".to_string(),
            FailureAxis::TpmUnavailable => "HALT_0x0006".to_string(),
            FailureAxis::UnauthorizedMode => "HALT_0x0007".to_string(),
        }
    }

    /// Severity classification (Critical, High, Medium, Low)
    pub fn severity(&self) -> &'static str {
        match self.axis {
            FailureAxis::TimingDriftFailure
            | FailureAxis::InternalInvariantBreach
            | FailureAxis::ExternalInjectionDetected => "Critical",
            FailureAxis::AuthorityInversionAttempt
            | FailureAxis::Axiom6_7Misalignment => "High",
            FailureAxis::Reference | FailureAxis::Feedback | FailureAxis::Coupling => "Medium",
            FailureAxis::Resolution
            | FailureAxis::TpmUnavailable
            | FailureAxis::UnauthorizedMode => "Low",
        }
    }
}
