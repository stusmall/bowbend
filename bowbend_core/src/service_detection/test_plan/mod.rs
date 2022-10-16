use std::{collections::BTreeMap, sync::Arc};

use crate::service_detection::framework::{PortToAnalyze, Rule, RuleId};

pub struct PortTestPlan {
    port_to_analyze: Arc<PortToAnalyze>,
    to_run: Vec<Box<dyn Rule>>,
    already_successfully_run: Vec<RuleId>,
    /// A list of rules that are blocked indexed by the first rule blocking
    /// them.
    blocked: BTreeMap<RuleId, Vec<Box<dyn Rule>>>,
}

impl PortTestPlan {
    pub fn new(port_to_analyze: Arc<PortToAnalyze>, rules: Vec<Box<dyn Rule>>) -> Self {
        rules.into_iter().fold(
            PortTestPlan {
                port_to_analyze,
                to_run: vec![],
                already_successfully_run: vec![],
                blocked: Default::default(),
            },
            |mut testplan, rule| {
                // Does the rule apply to the port we are planning for?
                let applicable_port = rule.port_hints().iter().any(|port_hint| {
                    port_hint
                        .port_range()
                        .contains(&testplan.port_to_analyze.port())
                });
                if applicable_port {
                    if let Some(first_depedency) = rule.dependencies().first() {
                        testplan
                            .blocked
                            .entry(first_depedency.clone())
                            .or_default()
                            .push(rule);
                        testplan
                    } else {
                        testplan.to_run.push(rule);
                        testplan
                    }
                } else {
                    testplan
                }
            },
        )
    }

    pub fn build_next_stage_plan(mut self, mut successfully_run: Vec<RuleId>) -> Self {
        self.to_run = Vec::new();
        // First let's get a list of new rules to evaluate
        let mut possibly_unblocked_rules = Vec::new();
        for rule_id in &successfully_run {
            if let Some(mut rules) = self.blocked.remove(rule_id) {
                possibly_unblocked_rules.append(&mut rules);
            }
        }
        // Add these successfully run rules to our list of already run rules
        self.already_successfully_run.append(&mut successfully_run);

        for possibly_unblocked_rule in possibly_unblocked_rules {
            let unsatisfied_deps = possibly_unblocked_rule
                .dependencies()
                .iter()
                .filter(|rule_id| !self.already_successfully_run.contains(rule_id))
                .cloned()
                .collect::<Vec<RuleId>>();
            if let Some(first_unmet_dep) = unsatisfied_deps.first() {
                // There was some unmet dep.  Let's file this away as blocked under the first
                // rule in the list.  When it is finished we will check again
                self.blocked
                    .entry(first_unmet_dep.clone().clone())
                    .or_default()
                    .push(possibly_unblocked_rule);
            } else {
                // There are no unmet deps, let's queue it to run.
                self.to_run.push(possibly_unblocked_rule);
            }
        }
        self
    }

    pub fn has_actions_to_run(&self) -> bool {
        !self.to_run.is_empty()
    }

    pub fn rules_to_run(&self) -> &[Box<dyn Rule>] {
        &self.to_run
    }
}
