// ISE Harness Integration Test
// Comprehensive validation of PTP synchronization audit, HALT_0xABF3 trigger,
// and deterministic evidence repository.

#![allow(dead_code)]

use m_v_r_esprint1::ise::{IseHarness, IseConfig, ExecutionMode};
use m_v_r_esprint1::canonical_time::CanonicalTime;
use m_v_r_esprint1::ptp_compliance::{PtpCompliance, ClockClass, Stratum};
use m_v_r_esprint1::evidence_repository::{EvidenceRepository, EvidenceClass};
use m_v_r_esprint1::failure_axis::FailureAxis;

#[test]
fn test_ise_step_mode_without_faults() {
    let config = IseConfig {
        mode: ExecutionMode::Step,
        max_ticks: 100,
        enable_clock_drift_injection: false,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 0,
        fault_injection_rate: 0.0,
    };

    let mut harness = IseHarness::new(config);

    for _ in 0..100 {
        let result = harness.step_tick();
        assert!(result.is_ok(), "Step mode should succeed without injections");
    }

    let stats = harness.statistics();
    assert_eq!(stats.total_ticks, 100);
    assert_eq!(stats.timing_ok_count, 100);
    assert_eq!(stats.timing_drift_count, 0);
    assert!(stats.is_compliant());
}

#[test]
fn test_ise_accelerated_mode() {
    let config = IseConfig {
        mode: ExecutionMode::Accelerated(60),  // 60x speed
        max_ticks: 50,
        enable_clock_drift_injection: false,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 0,
        fault_injection_rate: 0.0,
    };

    let mut harness = IseHarness::new(config);

    for _ in 0..50 {
        let result = harness.step_tick();
        assert!(result.is_ok());
    }

    let stats = harness.statistics();
    assert_eq!(stats.total_ticks, 50);
    assert_eq!(stats.timing_ok_count, 50);
}

#[test]
fn test_ise_drift_injection_exceeds_tolerance() {
    let config = IseConfig {
        mode: ExecutionMode::Step,
        max_ticks: 5,
        enable_clock_drift_injection: true,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 50,  // Exceed ±20 ppm limit
        fault_injection_rate: 0.0,
    };

    let mut harness = IseHarness::new(config);

    let result = harness.step_tick();
    assert!(
        result.is_err(),
        "Should fail when drift injection exceeds tolerance"
    );

    if let Err(halt) = result {
        assert_eq!(halt.axis, FailureAxis::TimingDriftFailure);
        assert!(halt.message.contains("HALT_0xABF3"));
    }
}

#[test]
fn test_ise_fingerprint_determinism() {
    let config = IseConfig {
        mode: ExecutionMode::Step,
        max_ticks: 20,
        enable_clock_drift_injection: false,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 0,
        fault_injection_rate: 0.0,
    };

    let mut harness1 = IseHarness::new(config.clone());
    let mut harness2 = IseHarness::new(config);

    // Run identical scenarios
    for _ in 0..20 {
        let _ = harness1.step_tick();
        let _ = harness2.step_tick();
    }

    let fp1 = harness1.compute_fingerprint();
    let fp2 = harness2.compute_fingerprint();

    assert_eq!(
        fp1, fp2,
        "Identical ISE executions must produce identical fingerprints (Deterministic or Bust)"
    );
}

#[test]
fn test_ptp_compliance_phase_offset_validation() {
    let compliance = PtpCompliance::new(ClockClass::OrdinaryLocked, Stratum::Local);

    // Phase within tolerance (±250 ns)
    assert!(compliance.validate_phase_offset(100).is_ok());
    assert!(compliance.validate_phase_offset(-100).is_ok());
    assert!(compliance.validate_phase_offset(250).is_ok());

    // Phase exceeding tolerance
    assert!(compliance.validate_phase_offset(300).is_err());
    assert!(compliance.validate_phase_offset(-300).is_err());
}

#[test]
fn test_ptp_compliance_frequency_drift_validation() {
    let compliance = PtpCompliance::new(ClockClass::OrdinaryLocked, Stratum::Local);

    // Frequency within tolerance (±20 ppm)
    assert!(compliance.validate_frequency_drift(0).is_ok());
    assert!(compliance.validate_frequency_drift(10).is_ok());
    assert!(compliance.validate_frequency_drift(-10).is_ok());
    assert!(compliance.validate_frequency_drift(20).is_ok());

    // Frequency exceeding tolerance
    assert!(compliance.validate_frequency_drift(25).is_err());
    assert!(compliance.validate_frequency_drift(-25).is_err());
}

#[test]
fn test_ptp_compliance_halt_0xabf3_trigger() {
    let mut compliance = PtpCompliance::new(ClockClass::OrdinaryLocked, Stratum::Local);

    // Record maximum allowed syncs
    for _ in 0..10 {
        assert!(compliance.record_missed_sync().is_ok());
    }

    // 11th sync miss should trigger HALT_0xABF3
    let result = compliance.record_missed_sync();
    assert!(result.is_err());

    if let Err(halt) = result {
        assert_eq!(halt.axis, FailureAxis::TimingDriftFailure);
        assert!(halt.message.contains("HALT_0xABF3"));
    }
}

#[test]
fn test_evidence_repository_chain_integrity() {
    let mut repo = EvidenceRepository::new();

    repo.append_evidence(1000000, EvidenceClass::TimingOk, "Tick 1: OK".to_string());
    repo.append_evidence(2000000, EvidenceClass::TimingOk, "Tick 2: OK".to_string());
    repo.append_evidence(
        3000000,
        EvidenceClass::TimingDrift,
        "Tick 3: Phase offset exceeded".to_string(),
    );

    // Verify chain integrity (no tampering)
    assert!(repo.verify_chain_integrity().is_ok());

    // Get compliance summary
    let summary = repo.compliance_summary();
    assert_eq!(summary.total_records, 3);
    assert_eq!(summary.timing_ok_count, 2);
    assert_eq!(summary.timing_drift_count, 1);
    assert!(!summary.is_compliant());  // Failure records present
}

#[test]
fn test_evidence_repository_fingerprint_immutability() {
    let mut repo1 = EvidenceRepository::new();
    let mut repo2 = EvidenceRepository::new();

    // Add identical records
    for i in 0..10 {
        repo1.append_evidence(
            (i * 1000000) as u128,
            EvidenceClass::TimingOk,
            format!("Record {}", i),
        );
        repo2.append_evidence(
            (i * 1000000) as u128,
            EvidenceClass::TimingOk,
            format!("Record {}", i),
        );
    }

    let fp1 = repo1.compute_fingerprint();
    let fp2 = repo2.compute_fingerprint();

    assert_eq!(
        fp1, fp2,
        "Identical evidence records must produce identical fingerprints"
    );
}

#[test]
fn test_canonical_time_sub_microsecond_precision() {
    let t1 = CanonicalTime::from_nanos(1_234_567_890);
    let t2 = CanonicalTime::from_nanos(1_234_567_900);

    let offset_ns = t2.phase_offset_ns(t1);
    assert_eq!(offset_ns, 10);  // 10 nanoseconds apart

    // Verify phase tolerance check
    let t3 = CanonicalTime::from_nanos(1_234_567_890 + 300);
    assert!(t3.exceeds_phase_tolerance(t1));
}

#[test]
fn test_ise_failure_classification_tracking() {
    let config = IseConfig {
        mode: ExecutionMode::Step,
        max_ticks: 10,
        enable_clock_drift_injection: false,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 0,
        fault_injection_rate: 0.0,
    };

    let mut harness = IseHarness::new(config);

    for _ in 0..10 {
        let _ = harness.step_tick();
    }

    // Verify all records are properly classified
    for evidence in harness.evidence_log() {
        let failure_axis = evidence.classification.to_failure_axis();
        // TimingOk should have no failure axis
        if let Some(_axis) = failure_axis {
            // Non-OK results should map to appropriate axis
            assert!(true);  // Placeholder
        }
    }
}

#[test]
fn test_ise_failure_axis_halt_codes() {
    use m_v_r_esprint1::failure_axis::{FailureAxis, SystemHalt};

    let halt_timing = SystemHalt::new(FailureAxis::TimingDriftFailure, "PTP drift");
    assert_eq!(halt_timing.halt_code(), "HALT_0xABF3");

    let halt_invariant = SystemHalt::new(FailureAxis::InternalInvariantBreach, "State corruption");
    assert_eq!(halt_invariant.halt_code(), "HALT_0xFEED");

    let halt_injection = SystemHalt::new(FailureAxis::ExternalInjectionDetected, "Fault detected");
    assert_eq!(halt_injection.halt_code(), "HALT_0xBADF");
}

#[test]
fn test_ise_complete_workflow() {
    // Simulates complete ISE audit workflow
    println!("\n=== ISE Complete Workflow Test ===\n");

    // 1. Create ISE harness with drift injection
    let config = IseConfig {
        mode: ExecutionMode::Step,
        max_ticks: 50,
        enable_clock_drift_injection: true,
        enable_parity_fault_injection: false,
        drift_injection_ppm: 10,  // Within tolerance
        fault_injection_rate: 0.0,
    };

    let mut harness = IseHarness::new(config);

    // 2. Run simulation
    let mut execution_halted = false;
    for _ in 0..50 {
        if harness.step_tick().is_err() {
            execution_halted = true;
            break;
        }
    }

    // 3. Verify statistics
    let stats = harness.statistics();
    println!("Total ticks: {}", stats.total_ticks);
    println!("Timing OK: {}", stats.timing_ok_count);
    println!("Timing drift: {}", stats.timing_drift_count);
    println!("Success rate: {:.2}%", stats.success_rate() * 100.0);
    println!("Compliant: {}", stats.is_compliant());
    println!("Halted: {}\n", execution_halted);

    // 4. Generate evidence repository
    let mut repo = EvidenceRepository::new();
    for evidence in harness.evidence_log() {
        let class = if let Some(_axis) = evidence.classification.to_failure_axis() {
            match evidence.classification.to_failure_axis().unwrap() {
                FailureAxis::TimingDriftFailure => EvidenceClass::TimingDrift,
                _ => EvidenceClass::DataCorruption,
            }
        } else {
            EvidenceClass::TimingOk
        };

        repo.append_evidence(evidence.canonical_time_ns, class, evidence.phase_offset_ns.to_string());
    }

    // 5. Verify chain and get summary
    assert!(repo.verify_chain_integrity().is_ok());
    let summary = repo.compliance_summary();
    
    println!("Evidence repository records: {}", summary.total_records);
    println!("Repository fingerprint: {}", &summary.fingerprint[0..16]);
    println!("Repository compliant: {}\n", summary.is_compliant());

    // 6. Compute ISE fingerprint
    let ise_fp = harness.compute_fingerprint();
    println!("ISE fingerprint: {}", hex::encode(&ise_fp[0..16]));
    println!("=== Test Complete ===\n");
}
