// ISE Runner CLI: Integration Simulation Environment Execution
// PHASE 3 IMPLEMENTATION
//
// Modes:
// - realtime:   Live PTP integration testing
// - accelerated: High-throughput replay (e.g., 60x speed)
// - step:       Controlled fault injection with manual time advancement
//
// Evidence Output:
// - JSON: Machine-readable execution trace and failure classification
// - Markdown: Human-readable ISE audit report
// - JSONL: Timeline of timing evidence for spreadsheet analysis

#![deny(unsafe_code)]

use m_v_r_esprint1::ise::{IseHarness, IseConfig, ExecutionMode, FailureClassification};
use m_v_r_esprint1::canonical_time::CanonicalTime;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::json;

#[derive(Debug, Clone)]
struct CliArgs {
    mode: ExecutionMode,
    max_ticks: u64,
    enable_drift_injection: bool,
    drift_ppm: i32,
    enable_parity_injection: bool,
    parity_fault_rate: f64,
    json_output: Option<PathBuf>,
    markdown_output: Option<PathBuf>,
    timeline_output: Option<PathBuf>,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Step,
            max_ticks: 1000,
            enable_drift_injection: false,
            drift_ppm: 0,
            enable_parity_injection: false,
            parity_fault_rate: 0.0,
            json_output: None,
            markdown_output: None,
            timeline_output: None,
        }
    }
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut parsed = CliArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                if i >= args.len() {
                    return Err("--mode requires an argument".to_string());
                }
                parsed.mode = match args[i].as_str() {
                    "realtime" => ExecutionMode::Realtime,
                    "step" => ExecutionMode::Step,
                    s if s.starts_with("accelerated:") => {
                        let factor_str = s.strip_prefix("accelerated:").unwrap();
                        let factor = factor_str
                            .parse::<u32>()
                            .map_err(|_| format!("Invalid acceleration factor: {}", factor_str))?;
                        ExecutionMode::Accelerated(factor)
                    }
                    _ => return Err(format!("Unknown mode: {}", args[i])),
                };
            }
            "--max-ticks" => {
                i += 1;
                if i >= args.len() {
                    return Err("--max-ticks requires an argument".to_string());
                }
                parsed.max_ticks = args[i]
                    .parse()
                    .map_err(|_| format!("Invalid tick count: {}", args[i]))?;
            }
            "--inject-drift" => {
                i += 1;
                if i >= args.len() {
                    return Err("--inject-drift requires a PPM value".to_string());
                }
                parsed.enable_drift_injection = true;
                parsed.drift_ppm = args[i]
                    .parse()
                    .map_err(|_| format!("Invalid PPM value: {}", args[i]))?;
            }
            "--inject-parity" => {
                i += 1;
                if i >= args.len() {
                    return Err("--inject-parity requires a rate (0-1)".to_string());
                }
                parsed.enable_parity_injection = true;
                parsed.parity_fault_rate = args[i]
                    .parse()
                    .map_err(|_| format!("Invalid fault rate: {}", args[i]))?;
            }
            "--json-output" => {
                i += 1;
                if i >= args.len() {
                    return Err("--json-output requires a path".to_string());
                }
                parsed.json_output = Some(PathBuf::from(&args[i]));
            }
            "--markdown-output" => {
                i += 1;
                if i >= args.len() {
                    return Err("--markdown-output requires a path".to_string());
                }
                parsed.markdown_output = Some(PathBuf::from(&args[i]));
            }
            "--timeline-output" => {
                i += 1;
                if i >= args.len() {
                    return Err("--timeline-output requires a path".to_string());
                }
                parsed.timeline_output = Some(PathBuf::from(&args[i]));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {}", args[i])),
        }
        i += 1;
    }

    Ok(parsed)
}

fn print_help() {
    println!("ISE Runner - Integration Simulation Environment CLI");
    println!();
    println!("USAGE:");
    println!("  ise_runner [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --mode <MODE>              Execution mode: 'realtime', 'step', or 'accelerated:N'");
    println!("  --max-ticks <N>            Maximum simulation ticks (default: 1000)");
    println!("  --inject-drift <PPM>       Inject PTP clock drift (parts per million)");
    println!("  --inject-parity <RATE>     Inject parity errors at given rate (0.0-1.0)");
    println!("  --json-output <PATH>       Write execution trace as JSON");
    println!("  --markdown-output <PATH>   Write audit report as Markdown");
    println!("  --timeline-output <PATH>   Write timing evidence as JSONL");
    println!("  --help, -h                 Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  # Run 100 ticks in step mode, output JSON report");
    println!("  ise_runner --mode step --max-ticks 100 --json-output report.json");
    println!();
    println!("  # Run accelerated 60x, inject 25 PPM drift, output all formats");
    println!("  ise_runner --mode accelerated:60 --inject-drift 25 \\");
    println!("    --json-output trace.json --markdown-output report.md \\");
    println!("    --timeline-output timeline.jsonl");
}

fn run_ise(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = IseConfig {
        mode: args.mode,
        max_ticks: args.max_ticks,
        enable_clock_drift_injection: args.enable_drift_injection,
        enable_parity_fault_injection: args.enable_parity_injection,
        drift_injection_ppm: args.drift_ppm,
        fault_injection_rate: args.parity_fault_rate,
    };

    let mut harness = IseHarness::new(config);

    println!("ISE Runner: Starting simulation");
    println!("Mode: {:?}", args.mode);
    println!("Max ticks: {}", args.max_ticks);
    if args.enable_drift_injection {
        println!("Clock drift injection: {} PPM", args.drift_ppm);
    }
    if args.enable_parity_injection {
        println!("Parity fault injection rate: {}", args.parity_fault_rate);
    }
    println!();

    let mut execution_halted = false;
    let mut halt_reason = String::new();

    for tick in 0..args.max_ticks {
        match harness.step_tick() {
            Ok(_) => {
                if tick % 100 == 0 {
                    println!("  Tick {}: OK", tick);
                }
            }
            Err(halt) => {
                execution_halted = true;
                halt_reason = halt.message.clone();
                println!("  Tick {}: HALT - {:?}: {}", tick, halt.axis, halt.message);
                break;
            }
        }
    }

    let stats = harness.statistics();

    println!();
    println!("=== ISE Execution Complete ===");
    println!("Total ticks executed: {}", stats.total_ticks);
    println!("Timing OK: {}", stats.timing_ok_count);
    println!("Timing drift: {}", stats.timing_drift_count);
    println!("Data corruption: {}", stats.data_corruption_count);
    println!("Injection detected: {}", stats.injection_detected_count);
    println!("Authority inversion: {}", stats.authority_inversion_count);
    println!("Success rate: {:.2}%", stats.success_rate() * 100.0);
    println!("Compliance: {}", if stats.is_compliant() {
        "PASS ✓"
    } else {
        "FAIL ✗"
    });

    if execution_halted {
        println!("Execution halted: {}", halt_reason);
    }

    // Generate outputs
    if let Some(json_path) = args.json_output {
        generate_json_report(&harness, &stats, json_path)?;
    }

    if let Some(md_path) = args.markdown_output {
        generate_markdown_report(&harness, &stats, execution_halted, &halt_reason, md_path)?;
    }

    if let Some(timeline_path) = args.timeline_output {
        generate_timeline_jsonl(&harness, timeline_path)?;
    }

    // Compute and display fingerprint
    let fingerprint = harness.compute_fingerprint();
    println!("Deterministic fingerprint: {}", hex::encode(&fingerprint));

    Ok(())
}

fn generate_json_report(
    harness: &IseHarness,
    stats: &m_v_r_esprint1::ise::IseStatistics,
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut evidence_items = Vec::new();

    for evidence in harness.evidence_log() {
        evidence_items.push(json!({
            "tick": evidence.tick,
            "canonical_time_ns": evidence.canonical_time_ns,
            "phase_offset_ns": evidence.phase_offset_ns,
            "freq_offset_ppm": evidence.freq_offset_ppm,
            "jitter_ns": evidence.jitter_ns,
            "classification": format!("{:?}", evidence.classification),
        }));
    }

    let report = json!({
        "execution": {
            "total_ticks": stats.total_ticks,
            "timing_ok": stats.timing_ok_count,
            "timing_drift": stats.timing_drift_count,
            "data_corruption": stats.data_corruption_count,
            "injection_detected": stats.injection_detected_count,
            "authority_inversion": stats.authority_inversion_count,
        },
        "evidence": evidence_items,
        "fingerprint": hex::encode(harness.compute_fingerprint()),
        "compliance": {
            "is_compliant": stats.is_compliant(),
            "success_rate": stats.success_rate(),
        }
    });

    fs::write(path, serde_json::to_string_pretty(&report)?)?;
    println!("JSON report written successfully");

    Ok(())
}

fn generate_markdown_report(
    harness: &IseHarness,
    stats: &m_v_r_esprint1::ise::IseStatistics,
    halted: bool,
    halt_reason: &str,
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut report = String::new();

    report.push_str("# ISE Harness Audit Report\n\n");
    report.push_str(&format!("**Generated:** {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));

    report.push_str("## Execution Summary\n\n");
    report.push_str(&format!("- **Total Ticks:** {}\n", stats.total_ticks));
    report.push_str(&format!("- **Timing OK:** {}\n", stats.timing_ok_count));
    report.push_str(&format!("- **Timing Drift:** {}\n", stats.timing_drift_count));
    report.push_str(&format!("- **Data Corruption:** {}\n", stats.data_corruption_count));
    report.push_str(&format!("- **Injection Detected:** {}\n", stats.injection_detected_count));
    report.push_str(&format!("- **Authority Inversion:** {}\n\n", stats.authority_inversion_count));

    report.push_str("## Compliance Status\n\n");
    report.push_str(&format!("- **Compliant:** {}\n", if stats.is_compliant() {
        "✓ PASS"
    } else {
        "✗ FAIL"
    }));
    report.push_str(&format!("- **Success Rate:** {:.2}%\n\n", stats.success_rate() * 100.0));

    if halted {
        report.push_str("## Halt Information\n\n");
        report.push_str(&format!("Execution was halted: {}\n\n", halt_reason));
    }

    report.push_str("## Evidence Log (Last 50 entries)\n\n");
    report.push_str("| Tick | Phase (ns) | Freq (ppm) | Classification |\n");
    report.push_str("|------|------------|-----------|----------------|\n");

    let evidence_list: Vec<_> = harness.evidence_log().iter().collect();
    let start_idx = if evidence_list.len() > 50 {
        evidence_list.len() - 50
    } else {
        0
    };

    for evidence in &evidence_list[start_idx..] {
        report.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            evidence.tick,
            evidence.phase_offset_ns,
            evidence.freq_offset_ppm,
            match &evidence.classification {
                FailureClassification::TimingOk { .. } => "TimingOK".to_string(),
                FailureClassification::TimingDriftPhase { .. } => "TimingDriftPhase".to_string(),
                FailureClassification::TimingDriftFrequency { .. } => "TimingDriftFreq".to_string(),
                FailureClassification::DataCorruption { .. } => "DataCorruption".to_string(),
                FailureClassification::InjectionDetected { .. } => "Injection".to_string(),
                FailureClassification::AuthorityInversion { .. } => "Authority".to_string(),
            }
        ));
    }

    report.push_str("\n## Deterministic Fingerprint\n\n");
    report.push_str(&format!("```\n{}\n```\n", hex::encode(harness.compute_fingerprint())));

    fs::write(path, report)?;
    println!("Markdown report written successfully");

    Ok(())
}

fn generate_timeline_jsonl(
    harness: &IseHarness,
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut timeline = String::new();

    for evidence in harness.evidence_log() {
        let line = json!({
            "tick": evidence.tick,
            "timestamp_ns": evidence.canonical_time_ns,
            "phase_offset_ns": evidence.phase_offset_ns,
            "freq_offset_ppm": evidence.freq_offset_ppm,
            "classification": format!("{:?}", evidence.classification),
        });
        timeline.push_str(&serde_json::to_string(&line)?);
        timeline.push('\n');
    }

    fs::write(path, timeline)?;
    println!("Timeline JSONL written successfully");

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let cli_args = match parse_args(&args[1..]) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Use --help for usage information");
            std::process::exit(1);
        }
    };

    if let Err(e) = run_ise(cli_args) {
        eprintln!("ISE Runner Error: {}", e);
        std::process::exit(1);
    }
}
