use super::ModelPoolSmokeAlignment;
use super::render::{bool_text, json_role_set, json_string_array, list_text, role_set_text};

impl ModelPoolSmokeAlignment {
    pub(crate) fn alignment_ok(&self) -> bool {
        self.alignment_ok
    }

    pub(crate) fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\",",
                "\"read_only\":true,",
                "\"launches_process\":false,",
                "\"sends_prompt\":false,",
                "\"alignment_ok\":{},",
                "\"manifest_roles\":{},",
                "\"status_roles\":{},",
                "\"unexpected_manifest_roles\":{},",
                "\"unexpected_status_roles\":{},",
                "\"manifest_quality_workers\":{},",
                "\"status_quality_workers\":{},",
                "\"extra_quality_12b_detected\":{},",
                "\"manifest_helper_workers\":{},",
                "\"status_helper_workers\":{},",
                "\"helper_target\":{},",
                "\"helper_worker_count_aligned\":{},",
                "\"missing_manifest_helper_roles\":{},",
                "\"missing_status_helper_roles\":{},",
                "\"missing_route_smoke_tasks\":{},",
                "\"unexpected_route_smoke_tasks\":{},",
                "\"route_smoke_count\":{},",
                "\"route_smoke_unique_tasks\":{},",
                "\"route_smoke_target\":{},",
                "\"route_smoke_count_aligned\":{},",
                "\"missing_status_roles\":{},",
                "\"unplanned_status_roles\":{},",
                "\"route_blocked_or_failed\":{}",
                "}}"
            ),
            bool_text(self.alignment_ok),
            json_role_set(&self.manifest_roles),
            json_role_set(&self.status_roles),
            json_string_array(&self.unexpected_manifest_roles),
            json_string_array(&self.unexpected_status_roles),
            self.manifest_quality_workers,
            self.status_quality_workers,
            bool_text(self.extra_quality_12b_detected),
            self.manifest_helper_workers,
            self.status_helper_workers,
            self.helper_target,
            bool_text(self.helper_worker_count_aligned),
            json_string_array(&self.missing_manifest_helper_roles),
            json_string_array(&self.missing_status_helper_roles),
            json_string_array(&self.missing_route_smoke_tasks),
            json_string_array(&self.unexpected_route_smoke_tasks),
            self.route_smoke_count,
            self.route_smoke_unique_tasks,
            self.route_smoke_target,
            bool_text(self.route_smoke_count_aligned),
            json_string_array(&self.missing_status_roles),
            json_string_array(&self.unplanned_status_roles),
            json_string_array(&self.route_blocked_or_failed),
        )
    }

    pub(crate) fn to_text(&self) -> String {
        [
            "model_pool_smoke_alignment".to_owned(),
            format!("alignment_ok={}", bool_text(self.alignment_ok)),
            format!("manifest_roles={}", role_set_text(&self.manifest_roles)),
            format!("status_roles={}", role_set_text(&self.status_roles)),
            format!(
                "unexpected_manifest_roles={}",
                list_text(&self.unexpected_manifest_roles)
            ),
            format!(
                "unexpected_status_roles={}",
                list_text(&self.unexpected_status_roles)
            ),
            format!(
                "manifest_quality_workers={} status_quality_workers={}",
                self.manifest_quality_workers, self.status_quality_workers
            ),
            format!(
                "extra_quality_12b_detected={}",
                bool_text(self.extra_quality_12b_detected)
            ),
            format!(
                "manifest_helper_workers={} status_helper_workers={} helper_target={}",
                self.manifest_helper_workers, self.status_helper_workers, self.helper_target
            ),
            format!(
                "helper_worker_count_aligned={}",
                bool_text(self.helper_worker_count_aligned)
            ),
            format!(
                "missing_manifest_helper_roles={}",
                list_text(&self.missing_manifest_helper_roles)
            ),
            format!(
                "missing_status_helper_roles={}",
                list_text(&self.missing_status_helper_roles)
            ),
            format!(
                "missing_route_smoke_tasks={}",
                list_text(&self.missing_route_smoke_tasks)
            ),
            format!(
                "unexpected_route_smoke_tasks={}",
                list_text(&self.unexpected_route_smoke_tasks)
            ),
            format!(
                "route_smoke_count={} route_smoke_unique_tasks={} route_smoke_target={} route_smoke_count_aligned={}",
                self.route_smoke_count,
                self.route_smoke_unique_tasks,
                self.route_smoke_target,
                bool_text(self.route_smoke_count_aligned)
            ),
            format!(
                "missing_status_roles={}",
                list_text(&self.missing_status_roles)
            ),
            format!(
                "unplanned_status_roles={}",
                list_text(&self.unplanned_status_roles)
            ),
            format!(
                "route_blocked_or_failed={}",
                list_text(&self.route_blocked_or_failed)
            ),
        ]
        .join("\n")
    }
}
