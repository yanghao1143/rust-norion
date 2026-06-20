use crate::{
    AdapterFailureReturnSummary, HardwareFailureReturnSummary, ManifestFailureReturnSummary,
    RuntimeFailureBatchSummary, RuntimeFailureReturnSummary, RuntimeFailureSummary,
    RuntimeKvExchangeFailureReturnSummary, RuntimeKvPersistenceFailureReturnSource,
    RuntimeKvPersistenceFailureReturnSummary, RuntimePlanningFailureReturnSummary,
    RuntimeRequestFailureReturnSummary, RuntimeResponseFailureReturnSummary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReturnFamily {
    RuntimeBoundary,
    Adapter,
    Hardware,
    Manifest,
    RuntimeKvExchange,
    RuntimeKvPersistence,
    RuntimePlanning,
    RuntimeRequest,
    RuntimeResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureReturnRoutingKey {
    pub family: FailureReturnFamily,
    pub source_label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureReturnRoutingSummary {
    pub family: FailureReturnFamily,
    pub source_label: &'static str,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
    pub can_return_runtime_failure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureReturnRoutingBatchSummary {
    pub route_count: usize,
    pub returnable_route_count: usize,
    pub accounting_problem_count: usize,
    pub failure_report_count: usize,
    pub blocker_component_count: usize,
    pub first_returnable_key: Option<FailureReturnRoutingKey>,
    pub has_runtime_boundary_route: bool,
    pub has_adapter_route: bool,
    pub has_hardware_route: bool,
    pub has_manifest_route: bool,
    pub has_runtime_kv_exchange_route: bool,
    pub has_runtime_kv_persistence_route: bool,
    pub has_runtime_planning_route: bool,
    pub has_runtime_request_route: bool,
    pub has_runtime_response_route: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureReturnRoutingSelection {
    pub batch: FailureReturnRoutingBatchSummary,
    pub decision: FailureReturnRoutingDecision,
    pub selected_route: Option<FailureReturnRoutingSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeKvPersistenceFailureReturnSelection {
    pub namespace_distribution: RuntimeKvPersistenceFailureReturnSummary,
    pub kv_fusion_persistence: RuntimeKvPersistenceFailureReturnSummary,
    pub namespace_route: FailureReturnRoutingSummary,
    pub fusion_route: FailureReturnRoutingSummary,
    pub selection: FailureReturnRoutingSelection,
    pub source_order_is_canonical: bool,
    pub selected_source: Option<RuntimeKvPersistenceFailureReturnSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReturnRoutingDecision {
    Continue,
    ReturnRuntimeFailure(FailureReturnRoutingKey),
    RepairAccounting { problem_count: usize },
}

impl FailureReturnFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::RuntimeBoundary => "runtime_boundary",
            Self::Adapter => "adapter",
            Self::Hardware => "hardware",
            Self::Manifest => "manifest",
            Self::RuntimeKvExchange => "runtime_kv_exchange",
            Self::RuntimeKvPersistence => "runtime_kv_persistence",
            Self::RuntimePlanning => "runtime_planning",
            Self::RuntimeRequest => "runtime_request",
            Self::RuntimeResponse => "runtime_response",
        }
    }
}

impl FailureReturnRoutingKey {
    pub fn family_label(self) -> &'static str {
        self.family.label()
    }
}

impl FailureReturnRoutingSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        family: FailureReturnFamily,
        source_label: &'static str,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
        commit_decision_accounting_consistent: bool,
        can_return_runtime_failure: bool,
    ) -> Self {
        Self {
            family,
            source_label,
            can_commit,
            should_return_failure,
            has_primary_failure_summary: primary_failure_summary.is_some(),
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
            commit_decision_accounting_consistent,
            can_return_runtime_failure,
        }
    }

    pub fn routing_key(self) -> FailureReturnRoutingKey {
        FailureReturnRoutingKey {
            family: self.family,
            source_label: self.source_label,
        }
    }

    pub fn matches_key(self, key: FailureReturnRoutingKey) -> bool {
        self.routing_key() == key
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_blocker_components())
    }

    pub fn can_route_runtime_failure(self) -> bool {
        self.can_return_runtime_failure && self.failure_return_accounting_is_consistent()
    }
}

impl FailureReturnRoutingBatchSummary {
    pub fn from_routes(routes: &[FailureReturnRoutingSummary]) -> Self {
        let mut summary = Self {
            route_count: routes.len(),
            returnable_route_count: 0,
            accounting_problem_count: 0,
            failure_report_count: 0,
            blocker_component_count: 0,
            first_returnable_key: None,
            has_runtime_boundary_route: false,
            has_adapter_route: false,
            has_hardware_route: false,
            has_manifest_route: false,
            has_runtime_kv_exchange_route: false,
            has_runtime_kv_persistence_route: false,
            has_runtime_planning_route: false,
            has_runtime_request_route: false,
            has_runtime_response_route: false,
        };

        for route in routes {
            if route.can_route_runtime_failure() {
                summary.returnable_route_count += 1;
                if summary.first_returnable_key.is_none() {
                    summary.first_returnable_key = Some(route.routing_key());
                }
            }
            if !route.failure_return_accounting_is_consistent() {
                summary.accounting_problem_count += 1;
            }
            summary.failure_report_count = summary
                .failure_report_count
                .saturating_add(route.failure_report_count);
            summary.blocker_component_count = summary
                .blocker_component_count
                .saturating_add(route.total_blocker_component_count);

            match route.family {
                FailureReturnFamily::RuntimeBoundary => {
                    summary.has_runtime_boundary_route = true;
                }
                FailureReturnFamily::Adapter => {
                    summary.has_adapter_route = true;
                }
                FailureReturnFamily::Hardware => {
                    summary.has_hardware_route = true;
                }
                FailureReturnFamily::Manifest => {
                    summary.has_manifest_route = true;
                }
                FailureReturnFamily::RuntimeKvExchange => {
                    summary.has_runtime_kv_exchange_route = true;
                }
                FailureReturnFamily::RuntimeKvPersistence => {
                    summary.has_runtime_kv_persistence_route = true;
                }
                FailureReturnFamily::RuntimePlanning => {
                    summary.has_runtime_planning_route = true;
                }
                FailureReturnFamily::RuntimeRequest => {
                    summary.has_runtime_request_route = true;
                }
                FailureReturnFamily::RuntimeResponse => {
                    summary.has_runtime_response_route = true;
                }
            }
        }

        summary
    }

    pub fn has_routes(self) -> bool {
        self.route_count > 0
    }

    pub fn has_returnable_routes(self) -> bool {
        self.returnable_route_count > 0
    }

    pub fn has_accounting_problems(self) -> bool {
        self.accounting_problem_count > 0
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.blocker_component_count > 0
    }

    pub fn route_accounting_is_consistent(self) -> bool {
        !self.has_accounting_problems()
            && (!self.has_failure_reports() || self.has_blocker_components())
            && self.has_returnable_routes() == self.first_returnable_key.is_some()
            && self.returnable_route_count <= self.route_count
            && self.accounting_problem_count <= self.route_count
    }

    pub fn can_select_runtime_failure_route(self) -> bool {
        self.has_returnable_routes()
            && !self.has_accounting_problems()
            && self.route_accounting_is_consistent()
    }

    pub fn routing_decision(self) -> FailureReturnRoutingDecision {
        if self.has_accounting_problems() || !self.route_accounting_is_consistent() {
            FailureReturnRoutingDecision::RepairAccounting {
                problem_count: self.accounting_problem_count,
            }
        } else if let Some(key) = self.first_returnable_key {
            FailureReturnRoutingDecision::ReturnRuntimeFailure(key)
        } else {
            FailureReturnRoutingDecision::Continue
        }
    }
}

impl FailureReturnRoutingSelection {
    pub fn from_routes(routes: &[FailureReturnRoutingSummary]) -> Self {
        let batch = FailureReturnRoutingBatchSummary::from_routes(routes);
        let decision = batch.routing_decision();
        let selected_route = decision.select_route(routes);

        Self {
            batch,
            decision,
            selected_route,
        }
    }

    pub fn should_continue(self) -> bool {
        self.decision.should_continue()
    }

    pub fn should_return_runtime_failure(self) -> bool {
        self.decision.should_return_runtime_failure()
    }

    pub fn should_repair_accounting(self) -> bool {
        self.decision.should_repair_accounting()
    }

    pub fn selected_key(self) -> Option<FailureReturnRoutingKey> {
        self.decision.return_key()
    }

    pub fn has_selected_route(self) -> bool {
        self.selected_route.is_some()
    }

    pub fn can_materialize_runtime_failure(self) -> bool {
        self.batch.can_select_runtime_failure_route()
            && self
                .selected_route
                .is_some_and(|route| route.can_route_runtime_failure())
    }
}

impl RuntimeKvPersistenceFailureReturnSelection {
    pub fn from_summaries(
        namespace_distribution: RuntimeKvPersistenceFailureReturnSummary,
        kv_fusion_persistence: RuntimeKvPersistenceFailureReturnSummary,
    ) -> Self {
        let namespace_route = FailureReturnRoutingSummary::from(namespace_distribution);
        let fusion_route = FailureReturnRoutingSummary::from(kv_fusion_persistence);
        let routes = [namespace_route, fusion_route];
        let selection = FailureReturnRoutingSelection::from_routes(&routes);
        let selected_source = selection
            .selected_key()
            .and_then(Self::source_from_persistence_label);
        let source_order_is_canonical = namespace_distribution.source
            == RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution
            && kv_fusion_persistence.source
                == RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence;

        Self {
            namespace_distribution,
            kv_fusion_persistence,
            namespace_route,
            fusion_route,
            selection,
            source_order_is_canonical,
            selected_source,
        }
    }

    pub fn should_continue(self) -> bool {
        self.source_order_is_canonical && self.selection.should_continue()
    }

    pub fn should_return_runtime_failure(self) -> bool {
        self.source_order_is_canonical && self.selection.should_return_runtime_failure()
    }

    pub fn should_repair_accounting(self) -> bool {
        !self.source_order_is_canonical || self.selection.should_repair_accounting()
    }

    pub fn can_materialize_runtime_failure(self) -> bool {
        self.source_order_is_canonical && self.selection.can_materialize_runtime_failure()
    }

    pub fn route_accounting_is_consistent(self) -> bool {
        self.source_order_is_canonical && self.selection.batch.route_accounting_is_consistent()
    }

    fn source_from_persistence_label(
        key: FailureReturnRoutingKey,
    ) -> Option<RuntimeKvPersistenceFailureReturnSource> {
        if key.family != FailureReturnFamily::RuntimeKvPersistence {
            return None;
        }

        match key.source_label {
            "kv_namespace_distribution" => {
                Some(RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution)
            }
            "kv_fusion_persistence" => {
                Some(RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence)
            }
            _ => None,
        }
    }
}

impl FailureReturnRoutingDecision {
    pub fn should_continue(self) -> bool {
        matches!(self, Self::Continue)
    }

    pub fn should_return_runtime_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure(_))
    }

    pub fn should_repair_accounting(self) -> bool {
        matches!(self, Self::RepairAccounting { .. })
    }

    pub fn return_key(self) -> Option<FailureReturnRoutingKey> {
        match self {
            Self::ReturnRuntimeFailure(key) => Some(key),
            _ => None,
        }
    }

    pub fn select_route(
        self,
        routes: &[FailureReturnRoutingSummary],
    ) -> Option<FailureReturnRoutingSummary> {
        let key = self.return_key()?;
        routes.iter().copied().find(|route| route.matches_key(key))
    }
}

macro_rules! impl_failure_return_routing_summary_from {
    ($summary:ty, $family:expr) => {
        impl From<$summary> for FailureReturnRoutingSummary {
            fn from(summary: $summary) -> Self {
                Self::new(
                    $family,
                    summary.source.label(),
                    summary.can_commit,
                    summary.should_return_failure,
                    summary.primary_failure_summary,
                    summary.failure_batch,
                    summary.failure_report_count,
                    summary.can_format_runtime_failures,
                    summary.total_blocker_component_count,
                    summary.commit_decision_accounting_consistent,
                    summary.can_return_runtime_failure(),
                )
            }
        }
    };
}

impl_failure_return_routing_summary_from!(
    RuntimeFailureReturnSummary,
    FailureReturnFamily::RuntimeBoundary
);
impl_failure_return_routing_summary_from!(
    AdapterFailureReturnSummary,
    FailureReturnFamily::Adapter
);
impl_failure_return_routing_summary_from!(
    HardwareFailureReturnSummary,
    FailureReturnFamily::Hardware
);
impl_failure_return_routing_summary_from!(
    ManifestFailureReturnSummary,
    FailureReturnFamily::Manifest
);
impl_failure_return_routing_summary_from!(
    RuntimeKvExchangeFailureReturnSummary,
    FailureReturnFamily::RuntimeKvExchange
);
impl_failure_return_routing_summary_from!(
    RuntimeKvPersistenceFailureReturnSummary,
    FailureReturnFamily::RuntimeKvPersistence
);
impl_failure_return_routing_summary_from!(
    RuntimeRequestFailureReturnSummary,
    FailureReturnFamily::RuntimeRequest
);
impl_failure_return_routing_summary_from!(
    RuntimeResponseFailureReturnSummary,
    FailureReturnFamily::RuntimeResponse
);

impl From<RuntimePlanningFailureReturnSummary> for FailureReturnRoutingSummary {
    fn from(summary: RuntimePlanningFailureReturnSummary) -> Self {
        Self::new(
            FailureReturnFamily::RuntimePlanning,
            summary.source.label(),
            summary.can_commit,
            summary.should_return_failure,
            summary.primary_failure_summary,
            summary.failure_batch,
            summary.failure_report_count,
            summary.can_format_runtime_failures,
            summary.total_problem_component_count,
            summary.commit_decision_accounting_consistent,
            summary.can_return_runtime_failure(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterFailureReturnSource, RuntimeFailureReport, RuntimeFailureReturnSource,
        RuntimeFailureReturnSummary, RuntimeKvExchangeFailureReturnSource,
        RuntimePlanningFailureReturnSource, RuntimePlanningFailureReturnSummary,
        RuntimeRequestFailureReturnSource, RuntimeResponseFailureReturnSource,
        RuntimeResponseFailureReturnSummary,
    };
    use crate::{KvFusionMergeSummary, KvNamespaceCounts};

    #[test]
    fn routing_summary_preserves_returnable_adapter_failure() {
        let failure = RuntimeFailureReport::contract_violation("adapter selection failed");
        let primary_summary = failure.failure_summary();
        let batch = RuntimeFailureReport::batch_summary(&[failure]);
        let adapter_summary = AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::AdapterSelection,
            false,
            true,
            Some(primary_summary),
            batch,
            1,
            true,
            2,
            true,
        );
        let routing = FailureReturnRoutingSummary::from(adapter_summary);

        assert_eq!(routing.family, FailureReturnFamily::Adapter);
        assert_eq!(routing.family.label(), "adapter");
        assert_eq!(routing.source_label, "adapter_selection");
        assert_eq!(
            routing.routing_key(),
            FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            }
        );
        assert_eq!(routing.primary_failure_summary, Some(primary_summary));
        assert!(routing.has_failure_reports());
        assert!(routing.has_blocker_components());
        assert!(routing.failure_return_accounting_is_consistent());
        assert!(routing.can_route_runtime_failure());
    }

    #[test]
    fn routing_summary_preserves_clean_request_acceptance_noop() {
        let request_summary = RuntimeRequestFailureReturnSummary::new(
            RuntimeRequestFailureReturnSource::RequestAcceptance,
            true,
            false,
            None,
            RuntimeFailureReport::batch_summary(&[]),
            0,
            false,
            0,
            true,
        );
        let routing = FailureReturnRoutingSummary::from(request_summary);

        assert_eq!(routing.family, FailureReturnFamily::RuntimeRequest);
        assert_eq!(routing.routing_key().family_label(), "runtime_request");
        assert_eq!(routing.source_label, "runtime_request_acceptance");
        assert!(!routing.has_failure_reports());
        assert!(!routing.has_blocker_components());
        assert!(routing.failure_return_accounting_is_consistent());
        assert!(!routing.can_route_runtime_failure());
    }

    #[test]
    fn routing_summary_rejects_inconsistent_public_accounting() {
        let failure = RuntimeFailureReport::kv_import("runtime kv import readiness failed");
        let primary_summary = failure.failure_summary();
        let batch = RuntimeFailureReport::batch_summary(&[failure]);
        let kv_summary = RuntimeKvExchangeFailureReturnSummary::new(
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
            false,
            true,
            Some(primary_summary),
            batch,
            0,
            true,
            1,
            true,
        );
        let routing = FailureReturnRoutingSummary::from(kv_summary);

        assert_eq!(routing.family, FailureReturnFamily::RuntimeKvExchange);
        assert_eq!(routing.source_label, "runtime_kv_import_readiness");
        assert!(!routing.failure_return_accounting_is_consistent());
        assert!(!routing.can_route_runtime_failure());
    }

    #[test]
    fn routing_batch_summary_selects_first_returnable_route() {
        let clean_request =
            FailureReturnRoutingSummary::from(RuntimeRequestFailureReturnSummary::new(
                RuntimeRequestFailureReturnSource::RequestAcceptance,
                true,
                false,
                None,
                RuntimeFailureReport::batch_summary(&[]),
                0,
                false,
                0,
                true,
            ));
        let adapter_failure = RuntimeFailureReport::contract_violation("adapter selection failed");
        let adapter_failure_summary = adapter_failure.failure_summary();
        let adapter = FailureReturnRoutingSummary::from(AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::AdapterSelection,
            false,
            true,
            Some(adapter_failure_summary),
            RuntimeFailureReport::batch_summary(&[adapter_failure]),
            1,
            true,
            2,
            true,
        ));
        let kv_failure = RuntimeFailureReport::kv_import("runtime kv import readiness failed");
        let kv_failure_summary = kv_failure.failure_summary();
        let kv = FailureReturnRoutingSummary::from(RuntimeKvExchangeFailureReturnSummary::new(
            RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
            false,
            true,
            Some(kv_failure_summary),
            RuntimeFailureReport::batch_summary(&[kv_failure]),
            1,
            true,
            1,
            true,
        ));
        let batch = FailureReturnRoutingBatchSummary::from_routes(&[clean_request, adapter, kv]);

        assert_eq!(batch.route_count, 3);
        assert_eq!(batch.returnable_route_count, 2);
        assert_eq!(batch.accounting_problem_count, 0);
        assert_eq!(batch.failure_report_count, 2);
        assert_eq!(batch.blocker_component_count, 3);
        assert_eq!(
            batch.first_returnable_key,
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            })
        );
        assert!(batch.has_runtime_request_route);
        assert!(batch.has_adapter_route);
        assert!(batch.has_runtime_kv_exchange_route);
        assert!(batch.route_accounting_is_consistent());
        assert!(batch.can_select_runtime_failure_route());
        assert_eq!(
            batch.routing_decision(),
            FailureReturnRoutingDecision::ReturnRuntimeFailure(FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            })
        );
        assert!(batch.routing_decision().should_return_runtime_failure());
        assert_eq!(
            batch.routing_decision().return_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            })
        );
        assert_eq!(
            adapter.matches_key(batch.routing_decision().return_key().unwrap()),
            true
        );
        assert_eq!(
            batch
                .routing_decision()
                .select_route(&[clean_request, adapter, kv]),
            Some(adapter)
        );
    }

    #[test]
    fn routing_batch_summary_blocks_selection_on_accounting_drift() {
        let failure = RuntimeFailureReport::kv_import("runtime kv import readiness failed");
        let failure_summary = failure.failure_summary();
        let drifted =
            FailureReturnRoutingSummary::from(RuntimeKvExchangeFailureReturnSummary::new(
                RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
                false,
                true,
                Some(failure_summary),
                RuntimeFailureReport::batch_summary(&[failure]),
                0,
                true,
                1,
                true,
            ));
        let batch = FailureReturnRoutingBatchSummary::from_routes(&[drifted]);

        assert_eq!(batch.route_count, 1);
        assert_eq!(batch.returnable_route_count, 0);
        assert_eq!(batch.accounting_problem_count, 1);
        assert_eq!(batch.first_returnable_key, None);
        assert!(batch.has_accounting_problems());
        assert!(!batch.route_accounting_is_consistent());
        assert!(!batch.can_select_runtime_failure_route());
        assert_eq!(
            batch.routing_decision(),
            FailureReturnRoutingDecision::RepairAccounting { problem_count: 1 }
        );
        assert!(batch.routing_decision().should_repair_accounting());
        assert_eq!(batch.routing_decision().select_route(&[drifted]), None);
    }

    #[test]
    fn routing_batch_summary_continues_for_clean_routes() {
        let clean_request =
            FailureReturnRoutingSummary::from(RuntimeRequestFailureReturnSummary::new(
                RuntimeRequestFailureReturnSource::RequestAcceptance,
                true,
                false,
                None,
                RuntimeFailureReport::batch_summary(&[]),
                0,
                false,
                0,
                true,
            ));
        let batch = FailureReturnRoutingBatchSummary::from_routes(&[clean_request]);

        assert_eq!(batch.route_count, 1);
        assert_eq!(batch.returnable_route_count, 0);
        assert_eq!(batch.accounting_problem_count, 0);
        assert_eq!(batch.first_returnable_key, None);
        assert!(batch.route_accounting_is_consistent());
        assert_eq!(
            batch.routing_decision(),
            FailureReturnRoutingDecision::Continue
        );
        assert!(batch.routing_decision().should_continue());
        assert_eq!(
            batch.routing_decision().select_route(&[clean_request]),
            None
        );
    }

    #[test]
    fn routing_selection_bundles_batch_decision_and_selected_route() {
        let clean_request =
            FailureReturnRoutingSummary::from(RuntimeRequestFailureReturnSummary::new(
                RuntimeRequestFailureReturnSource::RequestAcceptance,
                true,
                false,
                None,
                RuntimeFailureReport::batch_summary(&[]),
                0,
                false,
                0,
                true,
            ));
        let failure = RuntimeFailureReport::contract_violation("adapter selection failed");
        let failure_summary = failure.failure_summary();
        let adapter = FailureReturnRoutingSummary::from(AdapterFailureReturnSummary::new(
            AdapterFailureReturnSource::AdapterSelection,
            false,
            true,
            Some(failure_summary),
            RuntimeFailureReport::batch_summary(&[failure]),
            1,
            true,
            2,
            true,
        ));

        let selection = FailureReturnRoutingSelection::from_routes(&[clean_request, adapter]);

        assert_eq!(selection.batch.route_count, 2);
        assert_eq!(
            selection.decision,
            FailureReturnRoutingDecision::ReturnRuntimeFailure(FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            })
        );
        assert_eq!(selection.selected_route, Some(adapter));
        assert!(selection.should_return_runtime_failure());
        assert!(!selection.should_continue());
        assert!(!selection.should_repair_accounting());
        assert_eq!(
            selection.selected_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::Adapter,
                source_label: "adapter_selection"
            })
        );
        assert!(selection.has_selected_route());
        assert!(selection.can_materialize_runtime_failure());
    }

    #[test]
    fn routing_selection_continues_without_selected_route_for_clean_routes() {
        let clean_request =
            FailureReturnRoutingSummary::from(RuntimeRequestFailureReturnSummary::new(
                RuntimeRequestFailureReturnSource::RequestAcceptance,
                true,
                false,
                None,
                RuntimeFailureReport::batch_summary(&[]),
                0,
                false,
                0,
                true,
            ));

        let selection = FailureReturnRoutingSelection::from_routes(&[clean_request]);

        assert_eq!(selection.batch.route_count, 1);
        assert_eq!(selection.decision, FailureReturnRoutingDecision::Continue);
        assert_eq!(selection.selected_route, None);
        assert!(selection.should_continue());
        assert!(!selection.should_return_runtime_failure());
        assert!(!selection.should_repair_accounting());
        assert_eq!(selection.selected_key(), None);
        assert!(!selection.has_selected_route());
        assert!(!selection.can_materialize_runtime_failure());
    }

    #[test]
    fn routing_selection_repairs_accounting_without_materializing_failure() {
        let failure = RuntimeFailureReport::kv_import("runtime KV import readiness failed");
        let failure_summary = failure.failure_summary();
        let drifted =
            FailureReturnRoutingSummary::from(RuntimeKvExchangeFailureReturnSummary::new(
                RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
                false,
                true,
                Some(failure_summary),
                RuntimeFailureReport::batch_summary(&[failure]),
                0,
                true,
                1,
                true,
            ));

        let selection = FailureReturnRoutingSelection::from_routes(&[drifted]);

        assert_eq!(selection.batch.route_count, 1);
        assert_eq!(
            selection.decision,
            FailureReturnRoutingDecision::RepairAccounting { problem_count: 1 }
        );
        assert_eq!(selection.selected_route, None);
        assert!(!selection.should_continue());
        assert!(!selection.should_return_runtime_failure());
        assert!(selection.should_repair_accounting());
        assert_eq!(selection.selected_key(), None);
        assert!(!selection.has_selected_route());
        assert!(!selection.can_materialize_runtime_failure());
    }

    #[test]
    fn routing_selection_preserves_documented_migration_order() {
        let planning_failure =
            RuntimeFailureReport::contract_violation("runtime planning acceptance failed");
        let planning_failure_summary = planning_failure.failure_summary();
        let planning = FailureReturnRoutingSummary::from(RuntimePlanningFailureReturnSummary::new(
            RuntimePlanningFailureReturnSource::PlanningAcceptance,
            false,
            true,
            Some(planning_failure_summary),
            RuntimeFailureReport::batch_summary(&[planning_failure]),
            1,
            true,
            1,
            0,
            true,
        ));
        let request_failure = RuntimeFailureReport::kv_import("runtime request imported KV failed");
        let request_failure_summary = request_failure.failure_summary();
        let request = FailureReturnRoutingSummary::from(RuntimeRequestFailureReturnSummary::new(
            RuntimeRequestFailureReturnSource::RequestAcceptance,
            false,
            true,
            Some(request_failure_summary),
            RuntimeFailureReport::batch_summary(&[request_failure]),
            1,
            true,
            1,
            true,
        ));
        let response_failure =
            RuntimeFailureReport::kv_export("runtime response exported KV failed");
        let response_failure_summary = response_failure.failure_summary();
        let response = FailureReturnRoutingSummary::from(RuntimeResponseFailureReturnSummary::new(
            RuntimeResponseFailureReturnSource::ResponseAcceptance,
            false,
            true,
            Some(response_failure_summary),
            RuntimeFailureReport::batch_summary(&[response_failure]),
            1,
            true,
            1,
            true,
        ));
        let kv_failure = RuntimeFailureReport::kv_import("runtime KV import readiness failed");
        let kv_failure_summary = kv_failure.failure_summary();
        let kv_exchange =
            FailureReturnRoutingSummary::from(RuntimeKvExchangeFailureReturnSummary::new(
                RuntimeKvExchangeFailureReturnSource::RuntimeKvImportReadiness,
                false,
                true,
                Some(kv_failure_summary),
                RuntimeFailureReport::batch_summary(&[kv_failure]),
                1,
                true,
                1,
                true,
            ));
        let boundary_failure =
            RuntimeFailureReport::contract_violation("runtime boundary commit failed");
        let boundary_failure_summary = boundary_failure.failure_summary();
        let boundary = FailureReturnRoutingSummary::from(RuntimeFailureReturnSummary::new(
            RuntimeFailureReturnSource::BoundaryCommit,
            false,
            true,
            Some(boundary_failure_summary),
            RuntimeFailureReport::batch_summary(&[boundary_failure]),
            1,
            true,
            1,
            true,
        ));

        let selection = FailureReturnRoutingSelection::from_routes(&[
            planning,
            request,
            response,
            kv_exchange,
            boundary,
        ]);

        assert_eq!(selection.batch.route_count, 5);
        assert_eq!(selection.batch.returnable_route_count, 5);
        assert!(selection.batch.has_runtime_planning_route);
        assert!(selection.batch.has_runtime_request_route);
        assert!(selection.batch.has_runtime_response_route);
        assert!(selection.batch.has_runtime_kv_exchange_route);
        assert!(selection.batch.has_runtime_boundary_route);
        assert_eq!(
            selection.selected_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::RuntimePlanning,
                source_label: "runtime_planning_acceptance"
            })
        );
        assert_eq!(selection.selected_route, Some(planning));
        assert!(selection.can_materialize_runtime_failure());
    }

    #[test]
    fn routing_selection_prefers_namespace_distribution_before_fusion_persistence() {
        let expected_namespace = KvNamespaceCounts {
            runtime: 1,
            semantic: 0,
            gist: 0,
            agent: 0,
            custom: 0,
        };
        let actual_namespace = KvNamespaceCounts {
            runtime: 0,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        };
        let namespace_commit = expected_namespace
            .drift_summary(actual_namespace)
            .commit_summary();
        let namespace_route =
            FailureReturnRoutingSummary::from(namespace_commit.failure_return_summary());
        let fusion_commit = KvFusionMergeSummary {
            before: 1,
            after: 1,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary();
        let fusion_route =
            FailureReturnRoutingSummary::from(fusion_commit.failure_return_summary());

        let selection =
            FailureReturnRoutingSelection::from_routes(&[namespace_route, fusion_route]);

        assert_eq!(selection.batch.route_count, 2);
        assert_eq!(selection.batch.returnable_route_count, 2);
        assert!(selection.batch.has_runtime_kv_persistence_route);
        assert_eq!(
            selection.selected_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::RuntimeKvPersistence,
                source_label: "kv_namespace_distribution"
            })
        );
        assert_eq!(selection.selected_route, Some(namespace_route));
        assert!(selection.can_materialize_runtime_failure());
    }

    #[test]
    fn kv_persistence_selection_continues_for_clean_namespace_and_fusion() {
        let namespace = KvNamespaceCounts {
            runtime: 1,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        }
        .drift_summary(KvNamespaceCounts {
            runtime: 1,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        })
        .commit_summary()
        .failure_return_summary();
        let fusion = KvFusionMergeSummary {
            before: 2,
            after: 2,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 1,
            result_namespace_count: 2,
            namespace_counts: KvNamespaceCounts {
                runtime: 1,
                semantic: 1,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary()
        .failure_return_summary();

        let selection =
            RuntimeKvPersistenceFailureReturnSelection::from_summaries(namespace, fusion);

        assert!(selection.source_order_is_canonical);
        assert_eq!(selection.selection.batch.route_count, 2);
        assert_eq!(selection.selection.batch.returnable_route_count, 0);
        assert_eq!(selection.selected_source, None);
        assert!(selection.should_continue());
        assert!(!selection.should_return_runtime_failure());
        assert!(!selection.should_repair_accounting());
        assert!(selection.route_accounting_is_consistent());
        assert!(!selection.can_materialize_runtime_failure());
    }

    #[test]
    fn kv_persistence_selection_prefers_namespace_distribution_return() {
        let namespace = KvNamespaceCounts {
            runtime: 1,
            semantic: 0,
            gist: 0,
            agent: 0,
            custom: 0,
        }
        .drift_summary(KvNamespaceCounts {
            runtime: 0,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        })
        .commit_summary()
        .failure_return_summary();
        let fusion = KvFusionMergeSummary {
            before: 1,
            after: 1,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary()
        .failure_return_summary();

        let selection =
            RuntimeKvPersistenceFailureReturnSelection::from_summaries(namespace, fusion);

        assert!(selection.source_order_is_canonical);
        assert_eq!(selection.selection.batch.returnable_route_count, 2);
        assert_eq!(
            selection.selected_source,
            Some(RuntimeKvPersistenceFailureReturnSource::NamespaceDistribution)
        );
        assert_eq!(
            selection.selection.selected_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::RuntimeKvPersistence,
                source_label: "kv_namespace_distribution"
            })
        );
        assert!(!selection.should_continue());
        assert!(selection.should_return_runtime_failure());
        assert!(!selection.should_repair_accounting());
        assert!(selection.route_accounting_is_consistent());
        assert!(selection.can_materialize_runtime_failure());
    }

    #[test]
    fn kv_persistence_selection_returns_fusion_failure_after_clean_namespace_gate() {
        let namespace = KvNamespaceCounts {
            runtime: 1,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        }
        .drift_summary(KvNamespaceCounts {
            runtime: 1,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        })
        .commit_summary()
        .failure_return_summary();
        let fusion = KvFusionMergeSummary {
            before: 2,
            after: 1,
            merged_count: 1,
            skipped_count: 0,
            merge_fraction: 0.5,
            changed: true,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary()
        .failure_return_summary();

        let selection =
            RuntimeKvPersistenceFailureReturnSelection::from_summaries(namespace, fusion);

        assert!(selection.source_order_is_canonical);
        assert!(
            !selection
                .namespace_distribution
                .can_return_runtime_failure()
        );
        assert!(selection.kv_fusion_persistence.can_return_runtime_failure());
        assert_eq!(selection.selection.batch.returnable_route_count, 1);
        assert_eq!(
            selection.selected_source,
            Some(RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence)
        );
        assert_eq!(
            selection.selection.selected_key(),
            Some(FailureReturnRoutingKey {
                family: FailureReturnFamily::RuntimeKvPersistence,
                source_label: "kv_fusion_persistence"
            })
        );
        assert_eq!(
            selection.selection.selected_route,
            Some(selection.fusion_route)
        );
        assert!(!selection.should_continue());
        assert!(selection.should_return_runtime_failure());
        assert!(!selection.should_repair_accounting());
        assert!(selection.route_accounting_is_consistent());
        assert!(selection.can_materialize_runtime_failure());
    }

    #[test]
    fn kv_persistence_selection_repairs_noncanonical_source_order() {
        let namespace = KvNamespaceCounts {
            runtime: 1,
            semantic: 0,
            gist: 0,
            agent: 0,
            custom: 0,
        }
        .drift_summary(KvNamespaceCounts {
            runtime: 0,
            semantic: 1,
            gist: 0,
            agent: 0,
            custom: 0,
        })
        .commit_summary()
        .failure_return_summary();
        let fusion = KvFusionMergeSummary {
            before: 1,
            after: 1,
            merged_count: 0,
            skipped_count: 0,
            merge_fraction: 0.0,
            changed: false,
            skipped_due_to_limit: false,
            runtime_block_count: 1,
            non_runtime_block_count: 0,
            result_namespace_count: 1,
            namespace_counts: KvNamespaceCounts {
                runtime: 2,
                semantic: 0,
                gist: 0,
                agent: 0,
                custom: 0,
            },
        }
        .commit_summary()
        .failure_return_summary();

        let selection =
            RuntimeKvPersistenceFailureReturnSelection::from_summaries(fusion, namespace);

        assert!(!selection.source_order_is_canonical);
        assert_eq!(
            selection.selected_source,
            Some(RuntimeKvPersistenceFailureReturnSource::KvFusionPersistence)
        );
        assert!(!selection.should_continue());
        assert!(!selection.should_return_runtime_failure());
        assert!(selection.should_repair_accounting());
        assert!(!selection.route_accounting_is_consistent());
        assert!(!selection.can_materialize_runtime_failure());
    }
}
