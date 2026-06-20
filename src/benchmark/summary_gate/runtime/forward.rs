use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_forward_cases) = gate.min_runtime_forward_cases {
        let runtime_forward_cases = summary.runtime_forward_cases();
        if runtime_forward_cases < min_runtime_forward_cases {
            failures.push(format!(
                "runtime_forward_cases {} below minimum {}",
                runtime_forward_cases, min_runtime_forward_cases
            ));
        }
    }

    if let Some(min_runtime_forward_energy_cases) = gate.min_runtime_forward_energy_cases {
        let runtime_forward_energy_cases = summary.runtime_forward_energy_cases();
        if runtime_forward_energy_cases < min_runtime_forward_energy_cases {
            failures.push(format!(
                "runtime_forward_energy_cases {} below minimum {}",
                runtime_forward_energy_cases, min_runtime_forward_energy_cases
            ));
        }
    }

    if let Some(min_runtime_kv_influence_cases) = gate.min_runtime_kv_influence_cases {
        let runtime_kv_influence_cases = summary.runtime_kv_influence_cases();
        if runtime_kv_influence_cases < min_runtime_kv_influence_cases {
            failures.push(format!(
                "runtime_kv_influence_cases {} below minimum {}",
                runtime_kv_influence_cases, min_runtime_kv_influence_cases
            ));
        }
    }

    if let Some(min_runtime_architecture_cases) = gate.min_runtime_architecture_cases {
        let runtime_architecture_cases = summary.runtime_architecture_cases();
        if runtime_architecture_cases < min_runtime_architecture_cases {
            failures.push(format!(
                "runtime_architecture_cases {} below minimum {}",
                runtime_architecture_cases, min_runtime_architecture_cases
            ));
        }
    }

    if let Some(min_runtime_architecture_device_profiles) =
        gate.min_runtime_architecture_device_profiles
    {
        let runtime_architecture_device_profiles = summary.runtime_architecture_device_profiles();
        if runtime_architecture_device_profiles < min_runtime_architecture_device_profiles {
            failures.push(format!(
                "runtime_architecture_device_profiles {} below minimum {} devices={}",
                runtime_architecture_device_profiles,
                min_runtime_architecture_device_profiles,
                summary.runtime_architecture_evidence.devices_csv()
            ));
        }
    }

    if let Some(min_runtime_kv_precision_cases) = gate.min_runtime_kv_precision_cases {
        let runtime_kv_precision_cases = summary.runtime_kv_precision_cases();
        if runtime_kv_precision_cases < min_runtime_kv_precision_cases {
            failures.push(format!(
                "runtime_kv_precision_cases {} below minimum {}",
                runtime_kv_precision_cases, min_runtime_kv_precision_cases
            ));
        }
    }

    if let Some(min_runtime_layer_mode_cases) = gate.min_runtime_layer_mode_cases {
        let runtime_layer_mode_cases = summary.runtime_layer_mode_cases();
        if runtime_layer_mode_cases < min_runtime_layer_mode_cases {
            failures.push(format!(
                "runtime_layer_mode_cases {} below minimum {}",
                runtime_layer_mode_cases, min_runtime_layer_mode_cases
            ));
        }
    }

    if let Some(min_runtime_all_layer_mode_cases) = gate.min_runtime_all_layer_mode_cases {
        let runtime_all_layer_mode_cases = summary.runtime_all_layer_mode_cases();
        if runtime_all_layer_mode_cases < min_runtime_all_layer_mode_cases {
            failures.push(format!(
                "runtime_all_layer_mode_cases {} below minimum {}",
                runtime_all_layer_mode_cases, min_runtime_all_layer_mode_cases
            ));
        }
    }

    if let Some(min_runtime_global_layers) = gate.min_runtime_global_layers {
        let runtime_global_layers = summary.total_runtime_global_layers();
        if runtime_global_layers < min_runtime_global_layers {
            failures.push(format!(
                "runtime_global_layers {} below minimum {}",
                runtime_global_layers, min_runtime_global_layers
            ));
        }
    }

    if let Some(min_runtime_local_window_layers) = gate.min_runtime_local_window_layers {
        let runtime_local_window_layers = summary.total_runtime_local_window_layers();
        if runtime_local_window_layers < min_runtime_local_window_layers {
            failures.push(format!(
                "runtime_local_window_layers {} below minimum {}",
                runtime_local_window_layers, min_runtime_local_window_layers
            ));
        }
    }

    if let Some(min_runtime_convolutional_fusion_layers) =
        gate.min_runtime_convolutional_fusion_layers
    {
        let runtime_convolutional_fusion_layers =
            summary.total_runtime_convolutional_fusion_layers();
        if runtime_convolutional_fusion_layers < min_runtime_convolutional_fusion_layers {
            failures.push(format!(
                "runtime_convolutional_fusion_layers {} below minimum {}",
                runtime_convolutional_fusion_layers, min_runtime_convolutional_fusion_layers
            ));
        }
    }
}
