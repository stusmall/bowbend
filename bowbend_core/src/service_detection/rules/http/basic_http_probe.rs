use std::{collections::HashMap, ops::Range, sync::Arc};

use reqwest::{get, Client};
use tracing::{instrument, trace};

use crate::service_detection::{
    framework::{
        PortHint, PortLikeliness, PortToAnalyze, Rule, RuleClosure, RuleError, RuleId,
        RuleLoudness, RuleResult, RuleResults, ServiceDetectionConclusion,
    },
    rules::ssl::{BasicSSLProbe, BasicSSLProbeResult},
};

/// Rule to do a simple HTTP GET request for /.  It captures the output to be
/// analyzed by other rules.  This doesn't come to any conclusion on its own.
#[derive(Clone, Debug)]
pub struct BasicHttpGetProbe {}

impl BasicHttpGetProbe {
    /// Basic constructor for [`BasicHttpGetProbe`]
    pub fn new() -> Box<dyn Rule> {
        Box::new(BasicHttpGetProbe {})
    }
}

impl Rule for BasicHttpGetProbe {
    fn dependencies(&self) -> Vec<RuleId> {
        vec![RuleId::new::<BasicSSLProbe>()]
    }

    fn port_hints(&self) -> Vec<PortHint> {
        vec![
            PortHint::new(80, PortLikeliness::Standard),
            PortHint::new(443, PortLikeliness::Standard),
            PortHint::new(8080, PortLikeliness::Common),
            PortHint::new_from_range(
                Range {
                    start: 8081,
                    end: 8089,
                },
                PortLikeliness::Unusual,
            ),
        ]
    }

    fn loudness(&self) -> RuleLoudness {
        RuleLoudness::Standard
    }

    #[instrument(level = "trace", skip(self))]
    fn get_execution_method(&self) -> RuleClosure {
        async fn exec(
            target: Arc<PortToAnalyze>,
            results: Arc<RuleResults>,
        ) -> Result<Box<dyn RuleResult>, RuleError> {
            trace!("Grabbing SSL result");
            let ssl_result = results
                .get_results::<BasicSSLProbe, BasicSSLProbeResult>()
                .await;
            let socket_addr = target.get_socket_addr();
            let _permit = target.wait_for_clearance().await;
            let result = if ssl_result.ssl_enabled {
                let url = format!("https://{socket_addr}");
                Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .map_err(|e| RuleError::InternalRuleError(e.into()))?
                    .get(url)
                    .send()
                    .await
            } else {
                let url = format!("http://{socket_addr}");
                get(url).await
            }
            .map_err(|e| RuleError::InternalRuleError(e.into()))?;
            let headers =
                result
                    .headers()
                    .into_iter()
                    .fold(HashMap::new(), |mut collection, record| {
                        collection.insert(record.0.to_string(), record.1.as_bytes().to_vec());
                        collection
                    });
            Ok(Box::new(BasicHttpGetProbeResult {
                status_code: result.status().as_u16(),
                headers,
            }))
        }
        Box::new(
            move |target: Arc<PortToAnalyze>, results: Arc<RuleResults>| {
                Box::pin(exec(target, results))
            },
        )
    }
}

/// The captured result of a GET request to /.
#[derive(Debug, Clone)]
pub struct BasicHttpGetProbeResult {
    /// The status code returned by the result
    pub status_code: u16,
    /// The list of all headers included in the response.
    pub headers: HashMap<String, Vec<u8>>,
}

impl RuleResult for BasicHttpGetProbeResult {
    fn get_rule_id(&self) -> RuleId {
        RuleId::new::<BasicHttpGetProbe>()
    }

    fn get_conclusion(&self) -> Option<ServiceDetectionConclusion> {
        None
    }
}
