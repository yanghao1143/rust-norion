use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_kv_import_cases) = gate.min_runtime_kv_import_cases {
        let runtime_kv_import_cases = summary.runtime_kv_import_cases();
        if runtime_kv_import_cases < min_runtime_kv_import_cases {
            failures.push(format!(
                "runtime_kv_import_cases {} below minimum {}",
                runtime_kv_import_cases, min_runtime_kv_import_cases
            ));
        }
    }

    if let Some(min_runtime_kv_imported) = gate.min_runtime_kv_imported {
        let runtime_kv_imported = summary.total_runtime_kv_imported();
        if runtime_kv_imported < min_runtime_kv_imported {
            failures.push(format!(
                "runtime_kv_imported {} below minimum {}",
                runtime_kv_imported, min_runtime_kv_imported
            ));
        }
    }

    if let Some(min_runtime_kv_import_device_profiles) = gate.min_runtime_kv_import_device_profiles
    {
        let runtime_kv_import_device_profiles = summary.runtime_kv_import_device_profiles();
        if runtime_kv_import_device_profiles < min_runtime_kv_import_device_profiles {
            failures.push(format!(
                "runtime_kv_import_device_profiles {} below minimum {} devices={}",
                runtime_kv_import_device_profiles,
                min_runtime_kv_import_device_profiles,
                summary.runtime_kv_import_devices_csv()
            ));
        }
    }

    if let Some(min_runtime_kv_exported) = gate.min_runtime_kv_exported {
        let runtime_kv_exported = summary.total_runtime_kv_exported();
        if runtime_kv_exported < min_runtime_kv_exported {
            failures.push(format!(
                "runtime_kv_exported {} below minimum {}",
                runtime_kv_exported, min_runtime_kv_exported
            ));
        }
    }

    if let Some(min_runtime_kv_export_device_profiles) = gate.min_runtime_kv_export_device_profiles
    {
        let runtime_kv_export_device_profiles = summary.runtime_kv_export_device_profiles();
        if runtime_kv_export_device_profiles < min_runtime_kv_export_device_profiles {
            failures.push(format!(
                "runtime_kv_export_device_profiles {} below minimum {} devices={}",
                runtime_kv_export_device_profiles,
                min_runtime_kv_export_device_profiles,
                summary.runtime_kv_export_devices_csv()
            ));
        }
    }

    if let Some(min_runtime_kv_stored) = gate.min_runtime_kv_stored {
        let runtime_kv_stored = summary.total_runtime_kv_stored();
        if runtime_kv_stored < min_runtime_kv_stored {
            failures.push(format!(
                "runtime_kv_stored {} below minimum {}",
                runtime_kv_stored, min_runtime_kv_stored
            ));
        }
    }

    if let Some(min_runtime_kv_stored_device_profiles) = gate.min_runtime_kv_stored_device_profiles
    {
        let runtime_kv_stored_device_profiles = summary.runtime_kv_stored_device_profiles();
        if runtime_kv_stored_device_profiles < min_runtime_kv_stored_device_profiles {
            failures.push(format!(
                "runtime_kv_stored_device_profiles {} below minimum {} devices={}",
                runtime_kv_stored_device_profiles,
                min_runtime_kv_stored_device_profiles,
                summary.runtime_kv_stored_devices_csv()
            ));
        }
    }

    if let Some(min_runtime_kv_hold_cases) = gate.min_runtime_kv_hold_cases {
        let runtime_kv_hold_cases = summary.runtime_kv_hold_cases();
        if runtime_kv_hold_cases < min_runtime_kv_hold_cases {
            failures.push(format!(
                "runtime_kv_hold_cases {} below minimum {}",
                runtime_kv_hold_cases, min_runtime_kv_hold_cases
            ));
        }
    }

    if let Some(min_runtime_kv_held) = gate.min_runtime_kv_held {
        let runtime_kv_held = summary.total_runtime_kv_held();
        if runtime_kv_held < min_runtime_kv_held {
            failures.push(format!(
                "runtime_kv_held {} below minimum {}",
                runtime_kv_held, min_runtime_kv_held
            ));
        }
    }

    if let Some(min_runtime_kv_hold_device_profiles) = gate.min_runtime_kv_hold_device_profiles {
        let runtime_kv_hold_device_profiles = summary.runtime_kv_hold_device_profiles();
        if runtime_kv_hold_device_profiles < min_runtime_kv_hold_device_profiles {
            failures.push(format!(
                "runtime_kv_hold_device_profiles {} below minimum {} devices={}",
                runtime_kv_hold_device_profiles,
                min_runtime_kv_hold_device_profiles,
                summary.runtime_kv_hold_devices_csv()
            ));
        }
    }
}
