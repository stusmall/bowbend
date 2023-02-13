use std::sync::Arc;

use nom::{
    bytes::complete::{tag, take_until},
    IResult,
};
use tracing::instrument;

use crate::service_detection::{
    framework::{
        PortHint, PortToAnalyze, Rule, RuleClosure, RuleError, RuleId, RuleLoudness, RuleResult,
        RuleResults, ServiceDetectionCertainty, ServiceDetectionConclusion,
    },
    rules::http::basic_http_probe::{BasicHttpGetProbe, BasicHttpGetProbeResult},
};

/// This rule detects if an nginx instance is listening to the port.  It doesn't
/// do any new network requests.  It only examines the headers captured in
/// [`BasicHttpGetProbe`] and looks for nginx banners.  If possible it will also
/// extract the version number.
#[derive(Clone, Debug)]
pub struct NginxDetectionRule {}

impl NginxDetectionRule {
    /// Simple constructor for [`NginxDetectionRule`]
    pub fn new() -> Box<Self> {
        Box::new(NginxDetectionRule {})
    }
}

impl Rule for NginxDetectionRule {
    fn dependencies(&self) -> Vec<RuleId> {
        vec![RuleId::new::<BasicHttpGetProbe>()]
    }

    fn port_hints(&self) -> Vec<PortHint> {
        vec![PortHint::any()]
    }

    fn loudness(&self) -> RuleLoudness {
        RuleLoudness::Silent
    }

    #[instrument(level = "trace", skip(self))]
    fn get_execution_method(&self) -> RuleClosure {
        async fn exec(
            _: Arc<PortToAnalyze>,
            results: Arc<RuleResults>,
        ) -> Result<Box<dyn RuleResult>, RuleError> {
            let http_result = results
                .get_results::<BasicHttpGetProbe, BasicHttpGetProbeResult>()
                .await;
            Ok(Box::new(match http_result.headers.get("server") {
                Some(contents) => match String::from_utf8(contents.to_vec()) {
                    Ok(contents) => {
                        if let Ok((_, parsed)) = parse_nginx_server_header(&contents) {
                            NginxDetectionRuleResult {
                                conclusion: Some(ServiceDetectionConclusion {
                                    certainty: ServiceDetectionCertainty::Advertised,
                                    service_name: "nginx HTTP server".to_string(),
                                    service_version: Some(parsed.version.to_owned()),
                                }),
                            }
                        } else {
                            NginxDetectionRuleResult::no_conclusion()
                        }
                    }
                    Err(_) => NginxDetectionRuleResult::no_conclusion(),
                },
                None => NginxDetectionRuleResult::no_conclusion(),
            }))
        }
        Box::new(
            move |target: Arc<PortToAnalyze>, results: Arc<RuleResults>| {
                Box::pin(exec(target, results))
            },
        )
    }
}

/// The result from a run of [`NginxDetectionRule`].
#[derive(Debug)]
pub struct NginxDetectionRuleResult {
    conclusion: Option<ServiceDetectionConclusion>,
}

impl NginxDetectionRuleResult {
    fn no_conclusion() -> Self {
        Self { conclusion: None }
    }
}

impl RuleResult for NginxDetectionRuleResult {
    fn get_rule_id(&self) -> RuleId {
        RuleId::new::<NginxDetectionRule>()
    }

    fn get_conclusion(&self) -> Option<ServiceDetectionConclusion> {
        self.conclusion.clone()
    }
}

#[derive(Debug, PartialEq)]
struct NginxServerHeader<'a> {
    version: &'a str,
}

fn parse_nginx_server_header(input: &str) -> IResult<&str, NginxServerHeader> {
    // An extremely simple and wrong parser combinator for nginx server headers.  As
    // I get more examples I can expand and fix it, but I won't stress it for
    // now.  I'm really interested in just getting something very simple
    // running.
    let (input, _) = tag("nginx/")(input)?;
    let (input, version) = take_until(" ")(input)?;
    Ok((input, NginxServerHeader { version }))
}

#[test]
fn test_parsing_header() {
    assert_eq!(
        parse_nginx_server_header("nginx/1.18.0 (Ubuntu)")
            .unwrap()
            .1,
        NginxServerHeader { version: "1.18.0" }
    );
    assert!(parse_nginx_server_header("apache").is_err());
}
