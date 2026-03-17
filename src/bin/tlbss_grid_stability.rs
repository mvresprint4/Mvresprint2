#![deny(unsafe_code)]

use mvre_sprint_guardian::audit_guardian::AuditGuardian;
use mvre_sprint_guardian::tlbss_integrity_engine::{
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
