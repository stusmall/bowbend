//! The service detection framework.  This includes all rules, execution,
//! building of plans, framework and more.  There is only one entry point to the
//! system and everything else should be contained in this module.
use std::{future::ready, ops::Range, sync::Arc};

use framework::RuleError;
use futures::{stream::FuturesUnordered, Stream, StreamExt};
use tokio::sync::Semaphore;
use tracing::info;

use crate::{
    report::{PortStatus, Report},
    service_detection::{
        framework::{PortToAnalyze, RuleResult, RuleResults, ServiceDetectionConclusion},
        rules::get_all_rules,
        test_plan::PortTestPlan,
    },
    target::TargetInstance,
};

pub mod framework;
pub mod rules;
mod test_plan;

/// The one entry point to service detection.  It accepts a stream of reports,
/// runs service detection for it and then decorates them with the conclusions.
pub async fn run_service_detection_on_target(
    mut report_stream: impl Stream<Item = Report> + Unpin,
    semaphore: Arc<Semaphore>,
    throttle_range: Option<Range<u64>>,
) -> impl Stream<Item = Report> {
    let futures = FuturesUnordered::new();
    while let Some(mut report) = report_stream.next().await {
        if let (Some(instance), Ok(contents)) = (&report.instance, &mut report.contents) {
            if let Some(ports) = &mut contents.ports {
                for mut port in ports.values_mut() {
                    if port.status == PortStatus::Open {
                        let service_detection_output = run_service_detection_on_port(
                            instance.clone(),
                            port.port,
                            semaphore.clone(),
                            throttle_range.clone(),
                        )
                        .await;
                        info!("Output of service detection {:?}", service_detection_output);
                        port.service_detection_conclusions = Some(service_detection_output);
                    }
                }
            };
        } else {
            info!("Skipping service detection {}", report.target);
        }
        futures.push(ready(report))
    }
    futures
}

async fn run_service_detection_on_port(
    target_instance: TargetInstance,
    port: u16,
    semaphore: Arc<Semaphore>,
    throttle_range: Option<Range<u64>>,
) -> Vec<ServiceDetectionConclusion> {
    let port_to_analyze = PortToAnalyze::new(
        semaphore.clone(),
        throttle_range,
        target_instance.clone(),
        port,
    );
    let mut plan = PortTestPlan::new(port_to_analyze.clone(), get_all_rules());
    let port_to_analyze = port_to_analyze.clone();
    let rule_results = RuleResults::new();

    while plan.has_actions_to_run() {
        let futures: FuturesUnordered<_> = plan
            .rules_to_run()
            .iter()
            .map(|rule| {
                let execution_closure = rule.get_execution_method();
                execution_closure(port_to_analyze.clone(), rule_results.clone())
            })
            .collect();

        let result_batch: Vec<Result<Box<dyn RuleResult>, RuleError>> = futures.collect().await;

        let mut successfully_run = Vec::new();
        for result in result_batch {
            match result {
                Ok(result) => {
                    successfully_run.push(result.get_rule_id());
                    rule_results.insert_result(result).await;
                }
                Err(e) => {
                    tracing::error!("Rule failed to run {:?}", e);
                }
            }
        }

        plan = plan.build_next_stage_plan(successfully_run);
    }
    rule_results.get_conclusion().await
}
