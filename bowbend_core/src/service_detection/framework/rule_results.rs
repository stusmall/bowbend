use std::{collections::HashMap, fmt::Debug, sync::Arc};

use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::debug;

use crate::{
    service_detection::framework::{Rule, RuleId, ServiceDetectionConclusion},
    utils::downcast::*,
};

/// This represents the minimum required information we need for the result of a
/// rule run.  More specific information should be stored as members of the
/// concrete implementation.
pub trait RuleResult: 'static + Debug + Send + Sync + AsAny {
    /// Returns the [`RuleId`] of the Rule this result belongs to.  This isn't
    /// useful for the rule authors.  This is only useful for book keeping
    /// inside the framework
    fn get_rule_id(&self) -> RuleId;
    /// Return any conclusion that can be made about the service running on this
    /// port from this rule.  Often a rule is only used as an intermediate and
    /// will never have a conclusion.  Or the rule is meant to come to a
    /// conclusion but couldn't in this case.  In these cases just return [None]
    fn get_conclusion(&self) -> Option<ServiceDetectionConclusion>;
}

impl Downcast for dyn RuleResult {}

/// This is a container that packages up all dependent intermediate rule
/// results.  It is only guaranteed to hold rules included in the list returned
/// by `dependent_rules`.
pub struct RuleResults {
    store: RwLock<HashMap<RuleId, Box<dyn RuleResult>>>,
}

impl RuleResults {
    /// Basic constructor for [`RuleResults`].  It's only every used inside an
    /// [Arc] so we go ahead and return that.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: RwLock::new(HashMap::new()),
        })
    }

    /// Used internally by the framework to populate results.  This shouldn't
    /// ever be used by a rule
    pub async fn insert_result(&self, result: Box<dyn RuleResult>) {
        let rule_id = result.get_rule_id();
        debug!("Inserting results for rule {:?}", rule_id);
        self.store.write().await.insert(rule_id, result);
    }

    /// Get a typed result for some dependent rule.  This will panic if the rule
    /// attempts to access an intermediate that it didn't list as a
    /// dependency.
    pub async fn get_results<T1: Rule, T2: RuleResult>(&self) -> RwLockReadGuard<T2> {
        let read = self.store.read().await;
        let rule_id = RuleId::new::<T1>();
        debug!("Fetching results for rule {:?}", rule_id);
        RwLockReadGuard::map(read, |guard| {
            guard.get(&rule_id)
                .map(|result|{
                    result
                        .downcast_ref::<T2>()
                        .expect("The result stored for a give rule doesn't match our expected type.  This is an internal implementation bug with either the rule fetching the result or the one that created it")
                })
                .expect("A rule attempted to get the results for one its dependencies but it wasn't there")
        })
    }

    /// Walks through all results currently registers and returns all
    /// conclusions.  It doesn't try to pick our most certain conclusions or
    /// order them. It returns them all.
    pub async fn get_conclusion(&self) -> Vec<ServiceDetectionConclusion> {
        let lock = self.store.read().await;
        lock.values()
            .filter_map(|entry| entry.get_conclusion())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::service_detection::framework::{
        rule_results::{RuleResult, RuleResults},
        PortHint, Rule, RuleClosure, RuleId, RuleLoudness, ServiceDetectionCertainty,
        ServiceDetectionConclusion,
    };

    #[tokio::test]
    async fn test_insert_and_collect_results() {
        #[derive(Debug, Default)]
        struct TestRule {}

        impl Rule for TestRule {
            fn port_hints(&self) -> Vec<PortHint> {
                todo!()
            }

            fn loudness(&self) -> RuleLoudness {
                todo!()
            }

            fn get_execution_method(&self) -> RuleClosure {
                todo!()
            }
        }

        #[derive(Debug, Default)]
        struct TestRuleResult {}

        impl RuleResult for TestRuleResult {
            fn get_rule_id(&self) -> RuleId {
                RuleId::new::<TestRule>()
            }

            fn get_conclusion(&self) -> Option<ServiceDetectionConclusion> {
                Some(ServiceDetectionConclusion {
                    certainty: ServiceDetectionCertainty::Advertised,
                    service_name: String::from("test"),
                    service_version: None,
                })
            }
        }

        let result = Box::new(TestRuleResult::default());
        let results = RuleResults::new();
        results.insert_result(result).await;
        let _ = results.get_results::<TestRule, TestRuleResult>().await;
        assert_eq!(results.get_conclusion().await.len(), 1);
    }
}
