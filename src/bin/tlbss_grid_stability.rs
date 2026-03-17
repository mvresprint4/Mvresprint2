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

use m_v_r_esprint1::audit_guardian::AuditGuardian;
use m_v_r_esprint1::tlbss_integrity_engine::{
    PlantClass, TlbssConfig, TlbssIntegrityEngine, TriEntityState,
};

fn main() {
    let initial = TriEntityState::new(0, 0, 0).expect("valid binary init");
    let mut engine = TlbssIntegrityEngine::new(PlantClass::Solar, initial, TlbssConfig::default());
    let guardian = AuditGuardian::new(0.7);

    for t in 0..2_000u64 {
        let ex = [(t % 2) as u8, ((t / 2) % 2) as u8, ((t / 3) % 2) as u8];
        match engine.tick(ex) {
            Ok(rec) => {
                if rec.tick % 200 == 0 {
                    let cert = guardian.certify(&rec);
                    println!(
                        "tick={} l6={:.3} gsi={:.1} boundary={} admissible={}",
                        rec.tick,
                        rec.stability_index.l6_coherence,
                        rec.stability_index.score_0_to_100,
                        rec.boundary_condition,
                        cert.admissible
                    );
                }
                if let Some(alert) = rec.dimensional_transition {
                    println!(
                        "DIMENSIONAL_TRANSITION_ALERT tick={} window={}s",
                        alert.boundary_tick, alert.window_seconds
                    );
                }
            }
            Err(halt) => {
                eprintln!("HALT axis={:?} msg={}", halt.axis, halt.message);
                std::process::exit(1);
            }
        }
    }
}
