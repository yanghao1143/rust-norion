use rust_norion::{
    DevicePlanGateReport, HeuristicBackend, KvQuantBenchmarkSummary, LocalTransformerRuntime,
    NoironEngine, ProductionKernelConformanceGate, RuntimeBackend, RuntimeManifestDeviceGateReport,
    append_self_evolution_admission_trace_jsonl, evaluate_trace_schema_jsonl,
};

use crate::cli::args::Args;
use crate::cli::benchmark::{
    benchmark_self_evolution_admission_report, print_benchmark_summary,
    print_production_kernel_conformance_matrix_report, print_production_kernel_conformance_report,
    run_benchmark, run_benchmark_for_args, run_production_benchmark_all_devices,
    run_production_kernel_conformance_all_devices,
};
use crate::cli::device::{
    print_device_gate_report, print_device_matrix_and_exit, print_device_probe_report,
};
use crate::cli::experience_audit::{
    print_experience_cleanup_audit_report, run_experience_cleanup_audit,
};
use crate::cli::experience_hygiene::{
    print_experience_hygiene_quarantine_report, print_experience_hygiene_report,
    run_experience_hygiene_quarantine, run_experience_hygiene_report,
};
use crate::cli::experience_index::{
    print_experience_index_clean_gist_report, run_experience_index_add_clean_gist,
};
use crate::cli::experience_repair::{print_experience_repair_report, run_experience_repair};
use crate::cli::experience_retrieval::{
    print_experience_retrieval_report, run_experience_retrieval_report,
};
use crate::cli::kv_quant::print_kv_quant_gate_report;
use crate::cli::roundtrip::{
    print_persistent_roundtrip_matrix_report, print_persistent_roundtrip_report,
    run_persistent_roundtrip, run_persistent_roundtrip_all_devices,
};
use crate::cli::runtime_manifest::print_runtime_manifest_gate_report;
use crate::cli::self_goal_queue::{print_self_goal_queue_report, run_self_goal_queue_report};
use crate::cli::state::{
    print_state_inspection_gate_report, print_state_inspection_matrix_gate_report,
    print_state_inspection_report, run_state_inspection, run_state_inspection_all_devices,
};
use crate::cli::trace_schema::print_trace_schema_gate_report;
use crate::engine_config::configure_engine;
use crate::gemma_business::contract::record_gemma_business_smoke_contract;
use crate::gemma_business::cycle_smoke::run_gemma_business_cycle_smoke;
use crate::gemma_business::model_service_smoke::run_gemma_model_service_smoke;
use crate::gemma_business::paths::{prepare_gemma_business_smoke_paths, prune_gemma_smoke_runs};
use crate::gemma_business::preflight::{
    gemma_business_smoke_check_only_report, gemma_business_smoke_preflight_failures,
    print_gemma_business_cycle_smoke_preflight_failures,
    print_gemma_business_smoke_check_only_report, print_gemma_business_smoke_preflight_failures,
    print_gemma_model_service_smoke_preflight_failures,
};
use crate::gemma_business::regression::{
    evaluate_gemma_business_cycle_smoke_report_gate, evaluate_gemma_business_regression_gate,
    gemma_business_regression_report_path, print_gemma_business_cycle_smoke_report_gate,
    print_gemma_business_regression_gate,
};
use crate::gemma_business::smoke_gate::run_gemma_business_smoke_gates;
use crate::inference_output::print_inference_summary;
use crate::inference_runner::run_timed_inference_with_options;
use crate::model_service::server::run_model_service_for_args;

fn runtime_backend_for_args<R>(runtime: R, args: &Args) -> RuntimeBackend<R> {
    let backend = RuntimeBackend::new(runtime);
    if let Some(max_tokens) = args.max_tokens {
        backend.with_max_tokens(max_tokens)
    } else {
        backend
    }
}

pub(crate) fn run(args: Args) -> std::io::Result<()> {
    if args.list_devices {
        print_device_matrix_and_exit();
    }
    if args.probe_device {
        print_device_probe_report(&args);
        return Ok(());
    }
    if args.device_gate {
        let report = DevicePlanGateReport::evaluate();
        print_device_gate_report(&report);
        if !report.passed() {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.kv_quant_gate {
        let summary = KvQuantBenchmarkSummary::run_default();
        let gate_report = summary.evaluate(&args.kv_quant_gate());
        print_kv_quant_gate_report(&summary, &gate_report);
        if !gate_report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if let Some(path) = &args.gemma_business_regression_gate_path {
        let report_path = gemma_business_regression_report_path(path);
        let report = evaluate_gemma_business_regression_gate(&report_path)?;
        print_gemma_business_regression_gate(&report_path, &report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if let Some(path) = &args.gemma_business_cycle_smoke_report_gate_path {
        let report = evaluate_gemma_business_cycle_smoke_report_gate(path)?;
        print_gemma_business_cycle_smoke_report_gate(path, &report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.runtime_manifest_gate {
        let manifest = args.runtime_manifest();
        let validation = manifest.validate_for_production();
        let device_gate = RuntimeManifestDeviceGateReport::evaluate(
            &manifest,
            &args.runtime_manifest_device_plan(),
        );
        let all_devices_gate = if args.runtime_manifest_all_devices_gate {
            Some(DevicePlanGateReport::evaluate_runtime_manifest(&manifest))
        } else {
            None
        };
        print_runtime_manifest_gate_report(
            &manifest,
            &validation,
            &device_gate,
            all_devices_gate.as_ref(),
        );
        if !validation.passed()
            || !device_gate.passed()
            || all_devices_gate
                .as_ref()
                .map(|report| !report.passed())
                .unwrap_or(false)
        {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.production_kernel_conformance_all_devices_gate {
        let report = run_production_kernel_conformance_all_devices(&args);
        print_production_kernel_conformance_matrix_report(&report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.production_kernel_conformance_gate {
        let runtime = args.production_runtime()?;
        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());
        print_production_kernel_conformance_report(&report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.self_goal_queue {
        let report = run_self_goal_queue_report(&args)?;
        print_self_goal_queue_report(&report);
        if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
            let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
            print_trace_schema_gate_report(trace_schema_gate_path, &report);
            if !report.passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    if args.trace_schema_gate_path.is_some()
        && args.trace_path.is_none()
        && args.benchmark_path.is_none()
    {
        let path = args.trace_schema_gate_path.as_ref().unwrap();
        let report = evaluate_trace_schema_jsonl(path)?;
        print_trace_schema_gate_report(path, &report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }

    if args.experience_hygiene {
        if args.experience_hygiene_quarantine {
            let report = run_experience_hygiene_quarantine(&args)?;
            print_experience_hygiene_quarantine_report(&report);
        } else {
            let report = run_experience_hygiene_report(&args)?;
            print_experience_hygiene_report(&args, &report);
        }
        return Ok(());
    }

    if args.experience_repair {
        let report = run_experience_repair(&args)?;
        print_experience_repair_report(&report);
        return Ok(());
    }

    if args.experience_retrieval {
        let report = run_experience_retrieval_report(&args)?;
        print_experience_retrieval_report(&args, &report);
        return Ok(());
    }

    if args.experience_index_add_clean_gist {
        let report = run_experience_index_add_clean_gist(&args)?;
        print_experience_index_clean_gist_report(&report);
        return Ok(());
    }

    if args.experience_cleanup_audit {
        let report = run_experience_cleanup_audit(&args)?;
        print_experience_cleanup_audit_report(&report);
        return Ok(());
    }

    if args.benchmark_roundtrip && args.inspect_state {
        if args.benchmark_all_devices {
            let roundtrip_report = run_persistent_roundtrip_all_devices(&args)?;
            print_persistent_roundtrip_matrix_report(&args, &roundtrip_report);
            let inspect_report = run_state_inspection_all_devices(&args)?;
            print_state_inspection_matrix_gate_report(&args, &inspect_report);
            if !roundtrip_report.passed || !inspect_report.passed() {
                std::process::exit(2);
            }
        } else {
            let roundtrip_report = run_persistent_roundtrip(&args)?;
            print_persistent_roundtrip_report(&args, &roundtrip_report);
            let inspect_report = run_state_inspection(&args)?;
            print_state_inspection_report(&args, &inspect_report);
            let inspect_passed = if args.inspect_gate {
                let gate_report = inspect_report.evaluate(&args.state_inspection_gate());
                print_state_inspection_gate_report(&gate_report);
                gate_report.passed()
            } else {
                true
            };
            if !roundtrip_report.passed || !inspect_passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    if args.inspect_state {
        if args.benchmark_all_devices {
            let report = run_state_inspection_all_devices(&args)?;
            print_state_inspection_matrix_gate_report(&args, &report);
            if !report.passed() {
                std::process::exit(2);
            }
        } else {
            let report = run_state_inspection(&args)?;
            print_state_inspection_report(&args, &report);
            if args.inspect_gate {
                let gate_report = report.evaluate(&args.state_inspection_gate());
                print_state_inspection_gate_report(&gate_report);
                if !gate_report.passed() {
                    std::process::exit(2);
                }
            }
        }
        return Ok(());
    }

    if args.benchmark_roundtrip {
        if args.benchmark_all_devices {
            let report = run_persistent_roundtrip_all_devices(&args)?;
            print_persistent_roundtrip_matrix_report(&args, &report);
            if !report.passed {
                std::process::exit(2);
            }
        } else {
            let report = run_persistent_roundtrip(&args)?;
            print_persistent_roundtrip_report(&args, &report);
            if !report.passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    if args.gemma_business_smoke
        || args.gemma_business_cycle_smoke
        || args.gemma_model_service_smoke
    {
        let failures = gemma_business_smoke_preflight_failures(&args);
        if args.gemma_smoke_check_only {
            let report = gemma_business_smoke_check_only_report(&args, &failures);
            let passed = report.passed();
            print_gemma_business_smoke_check_only_report(&report);
            if !passed {
                std::process::exit(2);
            }
            return Ok(());
        }
        prepare_gemma_business_smoke_paths(&args)?;
        if !failures.is_empty() {
            if args.gemma_model_service_smoke {
                print_gemma_model_service_smoke_preflight_failures(&failures);
            } else if args.gemma_business_cycle_smoke {
                print_gemma_business_cycle_smoke_preflight_failures(&failures);
            } else {
                print_gemma_business_smoke_preflight_failures(&failures);
            }
            std::process::exit(2);
        }
    }

    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, &args);
    let replay_report = if args.replay_limit > 0 {
        Some(engine.replay_experience(args.replay_limit))
    } else {
        None
    };

    if args.gemma_business_cycle_smoke {
        let passed = run_gemma_business_cycle_smoke(engine, &args)?;
        if !passed {
            std::process::exit(2);
        }
        prune_gemma_smoke_runs(&args)?;
        return Ok(());
    }

    if args.gemma_model_service_smoke {
        let passed = run_gemma_model_service_smoke(engine, &args)?;
        if !passed {
            std::process::exit(2);
        }
        prune_gemma_smoke_runs(&args)?;
        return Ok(());
    }

    if args.serve {
        if let Some(replay_report) = &replay_report {
            println!("experience_replay: {}", replay_report.summary());
        }
        if args.production_runtime {
            let runtime = args.production_runtime()?;
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_model_service_for_args(&mut engine, &mut backend, &args)?;
        } else if let Some(runtime) = args.gemma_http_runtime()? {
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_model_service_for_args(&mut engine, &mut backend, &args)?;
        } else if let Some(runtime) = args.command_runtime() {
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_model_service_for_args(&mut engine, &mut backend, &args)?;
        } else if args.local_runtime {
            let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_model_service_for_args(&mut engine, &mut backend, &args)?;
        } else {
            let mut backend = HeuristicBackend;
            run_model_service_for_args(&mut engine, &mut backend, &args)?;
        }
        return Ok(());
    }

    if let Some(benchmark_path) = args.benchmark_path.clone() {
        let summary = if args.production_runtime {
            if args.benchmark_all_devices {
                run_production_benchmark_all_devices(&mut engine, &args, &benchmark_path)?
            } else {
                let runtime = args.production_runtime()?;
                let mut backend = runtime_backend_for_args(runtime, &args);
                run_benchmark(&mut engine, &mut backend, &benchmark_path)?
            }
        } else if let Some(runtime) = args.gemma_http_runtime()? {
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        } else if let Some(runtime) = args.command_runtime() {
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        } else if args.local_runtime {
            let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
            let mut backend = runtime_backend_for_args(runtime, &args);
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        } else {
            let mut backend = HeuristicBackend;
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        };
        engine.save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        let gate_report = if args.benchmark_gate_enabled {
            Some(summary.evaluate(&args.benchmark_gate()))
        } else {
            None
        };
        let self_evolution_admission_report = gate_report.as_ref().map(|report| {
            benchmark_self_evolution_admission_report(
                format!("benchmark:{}", benchmark_path.display()),
                &engine,
                &summary,
                report,
                args.profile,
            )
        });
        if let Some(report) = self_evolution_admission_report.as_ref() {
            append_self_evolution_admission_trace_jsonl(&benchmark_path, report)?;
        }
        print_benchmark_summary(
            &args,
            &benchmark_path,
            &summary,
            gate_report.as_ref(),
            self_evolution_admission_report.as_ref(),
        );
        if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
            let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
            print_trace_schema_gate_report(trace_schema_gate_path, &report);
            if !report.passed {
                std::process::exit(2);
            }
        }
        if let Some(report) = gate_report
            && !report.passed
        {
            std::process::exit(2);
        }
        return Ok(());
    }

    let timed_outcome = if args.production_runtime {
        let runtime = args.production_runtime()?;
        let mut backend = runtime_backend_for_args(runtime, &args);
        run_timed_inference_with_options(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.max_tokens,
            args.trace_path.as_ref(),
            None,
        )?
    } else if let Some(runtime) = args.command_runtime() {
        let mut backend = runtime_backend_for_args(runtime, &args);
        run_timed_inference_with_options(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.max_tokens,
            args.trace_path.as_ref(),
            None,
        )?
    } else if let Some(runtime) = args.gemma_http_runtime()? {
        let mut backend = runtime_backend_for_args(runtime, &args);
        run_timed_inference_with_options(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.max_tokens,
            args.trace_path.as_ref(),
            None,
        )?
    } else if args.local_runtime {
        let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
        let mut backend = runtime_backend_for_args(runtime, &args);
        run_timed_inference_with_options(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.max_tokens,
            args.trace_path.as_ref(),
            None,
        )?
    } else {
        let mut backend = HeuristicBackend;
        run_timed_inference_with_options(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.max_tokens,
            args.trace_path.as_ref(),
            None,
        )?
    };
    if args.gemma_business_smoke {
        record_gemma_business_smoke_contract(
            &mut engine,
            &timed_outcome.outcome,
            args.trace_path.as_ref(),
        )?;
    }
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    print_inference_summary(&args, &timed_outcome, replay_report.as_ref())?;
    if args.gemma_business_smoke {
        let passed = run_gemma_business_smoke_gates(&args, &timed_outcome.outcome)?;
        if !passed {
            std::process::exit(2);
        }
        prune_gemma_smoke_runs(&args)?;
    } else if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
        let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
        print_trace_schema_gate_report(trace_schema_gate_path, &report);
        if !report.passed {
            std::process::exit(2);
        }
    }

    Ok(())
}
