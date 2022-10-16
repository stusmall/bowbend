//! This contains everything related to probing TLS/SSL on a port.  This won't
//! actually discover any services, it will just help guide other rules

use std::{pin::Pin, sync::Arc};

use openssl::ssl::{SslConnector, SslMethod};
use tokio::net::TcpStream;
use tokio_openssl::SslStream;
use tracing::instrument;

use crate::service_detection::framework::{
    PortHint, PortToAnalyze, Rule, RuleClosure, RuleError, RuleId, RuleLoudness, RuleResult,
    RuleResults, ServiceDetectionConclusion,
};

/// This is a very simple probe to see if SSL or TLS is enabled on the port.  It
/// doesn't capture cipher or protocol versions but can easily be updated to.
#[derive(Clone, Debug)]
pub struct BasicSSLProbe {}

impl BasicSSLProbe {
    /// Basic constructor for [`BasicSSLProbe`]
    pub fn new() -> Box<dyn Rule> {
        Box::new(BasicSSLProbe {})
    }
}

impl Rule for BasicSSLProbe {
    fn port_hints(&self) -> Vec<PortHint> {
        vec![PortHint::any()]
    }

    fn loudness(&self) -> RuleLoudness {
        RuleLoudness::Standard
    }

    #[instrument(level = "trace", skip(self))]
    fn get_execution_method(&self) -> RuleClosure {
        async fn exec(
            target: Arc<PortToAnalyze>,
            _: Arc<RuleResults>,
        ) -> Result<Box<dyn RuleResult>, RuleError> {
            let addr = target.get_socket_addr();
            let _permit = target.wait_for_clearance().await;
            let stream = TcpStream::connect(&addr).await?;
            let builder = SslConnector::builder(SslMethod::tls()).unwrap();
            let mut stream = builder
                .build()
                .configure()
                .and_then(|config| config.into_ssl(&target.get_hostname()))
                .and_then(|ssl| SslStream::new(ssl, stream))
                .map_err(|e| RuleError::InternalRuleError(e.into()))?;
            Ok(Box::new(BasicSSLProbeResult {
                ssl_enabled: Pin::new(&mut stream).connect().await.is_ok(),
            }))
        }
        Box::new(
            move |target: Arc<PortToAnalyze>, results: Arc<RuleResults>| {
                Box::pin(exec(target, results))
            },
        )
    }
}

/// The results of the rule [`BasicSSLProbe`]
#[derive(Debug, Clone)]
pub struct BasicSSLProbeResult {
    /// Is TLS or SSL enabled on this port?
    pub ssl_enabled: bool,
}

impl RuleResult for BasicSSLProbeResult {
    fn get_rule_id(&self) -> RuleId {
        RuleId::new::<BasicSSLProbe>()
    }

    fn get_conclusion(&self) -> Option<ServiceDetectionConclusion> {
        None
    }
}
