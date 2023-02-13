//! This is core of the framework for managing rule detection.  It provides all
//! types that help make up rules, assist in their scheduling and running.

mod error;
mod rule_results;

use std::{
    any::TypeId, fmt::Debug, future::Future, net::SocketAddr, ops::Range, pin::Pin, sync::Arc,
    time::Duration,
};

pub use error::RuleError;
use rand::{rngs::StdRng, Rng, SeedableRng};
pub use rule_results::{RuleResult, RuleResults};
use tokio::{
    sync::{AcquireError, Semaphore, SemaphorePermit},
    time::sleep,
};

use crate::target::TargetInstance;

/// This is the type all rule execution should conform to.  This just the
/// unprettified form of `async fn (Arc<PortToAnalyze>, Arc<RuleResults>) ->
/// Result<Box<dyn RuleResult>, RuleError>> + Send>`.  [`PortToAnalyze`] gives
/// the rule all the information it needs about the target.  [`RuleResults`]
/// allows it to access the results of any dependencies.
pub type RuleClosure = Box<
    dyn Fn(
        Arc<PortToAnalyze>,
        Arc<RuleResults>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn RuleResult>, RuleError>> + Send>>,
>;

/// This contains all the information needed about the target and to form our
/// requests.
#[derive(Clone)]
pub struct PortToAnalyze {
    semaphore: Arc<Semaphore>,
    throttle_range: Option<Range<u64>>,
    target_instance: TargetInstance,
    port: u16,
}

impl PortToAnalyze {
    /// Simple constructor for [`PortToAnalyze`]
    pub fn new(
        semaphore: Arc<Semaphore>,
        throttle_range: Option<Range<u64>>,
        target_instance: TargetInstance,
        port: u16,
    ) -> Arc<Self> {
        Arc::new(PortToAnalyze {
            semaphore,
            throttle_range,
            target_instance,
            port,
        })
    }

    /// This pauses until we are clear to make another request on this port.  It
    /// will check both the throttle settings from the user and claim a
    /// semaphore permit to another another in flight ticket.  The permit
    /// should be kept in scope for the duration of the request but should
    /// be dropped when it is done.  If it isn't the the rule could accumulate
    /// more than one permit, artificially limiting the number of requests
    /// allowed in flight.
    pub async fn wait_for_clearance(&self) -> Result<SemaphorePermit<'_>, AcquireError> {
        if let Some(ref throttle_range) = self.throttle_range {
            sleep(Duration::from_millis(
                StdRng::from_entropy().gen_range(throttle_range.clone()),
            ))
            .await;
        }
        self.semaphore.acquire().await
    }

    /// Build the hostname to use on any probes on the target.  If it is a
    /// hostname, we will use that otherwise it will be the bare IP.  An example
    /// of when this is valuable is in the HTTP Host header.
    pub fn get_hostname(&self) -> String {
        match &self.target_instance {
            TargetInstance::IP(ip) => ip.to_string(),
            TargetInstance::Network { instance_ip, .. } => instance_ip.to_string(),
            TargetInstance::Hostname { hostname, .. } => hostname.to_string(),
        }
    }

    /// Get a usable [`SocketAddr`] for requests.
    pub fn get_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.target_instance.get_ip(), self.port)
    }

    /// Get the port to probe
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// A automatically derived unique identifier for any rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct RuleId {
    // Right now this will use the [`TypeId`] since all rules are 'static but I
    // think in the future we might not be able to lean on this.  We might
    // load rules through plugins etc.  Keeping this will encapsulated
    // should help us refactor when the time comes.
    internal: TypeId,
}

impl RuleId {
    /// Simple constructor for [`RuleId`]
    pub fn new<T: Rule>() -> Self {
        Self {
            internal: TypeId::of::<T>(),
        }
    }
}

/// A hint from a rule giving guidance on what ports it should be run against/
pub struct PortHint {
    /// The port or ports covered by this hint
    ports: Range<u16>,
    /// The likelihood that this service will be on the ports covered by this
    /// hint.
    #[allow(dead_code)]
    likeliness: PortLikeliness,
}

impl PortHint {
    /// Create a hint for one port.
    pub fn new(port: u16, likeliness: PortLikeliness) -> Self {
        Self {
            ports: Range {
                start: port,
                end: port + 1,
            },
            likeliness,
        }
    }

    /// Create a hint that covers a range of ports
    pub fn new_from_range(ports: Range<u16>, likeliness: PortLikeliness) -> Self {
        Self { ports, likeliness }
    }

    /// This rule can apply to any port with equal likeliness.  The SSL probe is
    /// great example of this.
    pub fn any() -> Self {
        Self {
            ports: Range {
                // This will technically allow the rule to apply to port 0.  This is a weird thing
                // to scan, I don't know why someone would want to.  I guess this is more an issue
                // to think about when a user sets the ports to scan than when we are deciding what
                // rules to apply
                start: u16::MIN,
                end: u16::MAX,
            },
            likeliness: PortLikeliness::Standard,
        }
    }

    /// Returns the ports covered by this hint in a half open range.  Even when
    /// the hint only covers one port, it is still expressed as a range.  For
    /// example, if the hint covers port 80 it will be expressed by the range
    /// [80, 81)
    pub fn port_range(&self) -> &Range<u16> {
        &self.ports
    }
}

/// This allows a rule to say how likely it expects the covered service to be on
/// any given port. The system will use these hints to help decide when to apply
/// each rule.  On quicker, more stealthy scans we will stick to more likely
/// rules on more standard ports.  If we are running a more comprehensive scan
/// we might start include running rules on `Unusual` or even `Rare` ports
/// included in the scan.
#[allow(dead_code)]
pub enum PortLikeliness {
    /// This is the default ports.  If this protocol had an RFC, then this port
    /// is mentioned in it. The majority of implementations will default to
    /// this.  Think 80, 443 for HTTP(S) or 22 for SSH.
    Standard,
    /// This is something that is still pretty common but isn't exactly
    /// standard.  Maybe it isn't mentioned in the RFC and won't be a
    /// default for most implementations but it is recognized as
    /// normal.  Think 8080 for HTTP
    Common,
    /// This happens, maybe only in a few well known implementations.  It isn't
    /// super unheard of, but it wouldn't be your first guess but you won't
    /// be surprised to see it.  Maybe it's only used because of the
    /// `Common` options already have something bound to them.  Think 8081 for
    /// HTTP
    Unusual,
    /// This part assignment exists.  Some people do it but it's weird.  This
    /// likeliness flag is just a tiny step above picking a random port to
    /// bind a service to.  It is like putting ssh on 1337 or something
    /// tacky like that.
    Rare,
}

/// The base trait for a rule.  This can either be a final rule that enumerates
/// a service or it could be an intermediate rule that is used to perform some
/// reusable action.
pub trait Rule: Debug + Send + Sync + 'static {
    /// The unique ID for the rule.  This is automatically derive and we really
    /// should never override this.
    fn rule_id(&self) -> RuleId {
        RuleId {
            internal: TypeId::of::<Self>(),
        }
    }

    /// The list of IDs for all rules this rule depends on.  By default a rule
    /// depends on no other rules
    fn dependencies(&self) -> Vec<RuleId> {
        Default::default()
    }

    /// The list of port hints for this rule.  This includes the list of
    /// possible ports and also their likeliness.  The framework uses this
    /// to decide what rules to use.
    fn port_hints(&self) -> Vec<PortHint>;

    /// How much network traffic can we expect this rule to generate?
    fn loudness(&self) -> RuleLoudness;

    /// Does the required privilege access to run?  To be more specific, does
    /// this rule try to open a raw socket? Rules requiring high access than
    /// available will be pruned.
    fn requires_privileged_access(&self) -> bool {
        false
    }

    /// This is the meat of the rule.  The closure returned from this before is
    /// what will be run to evaluate gathered evidence or run probes to
    /// gather more.
    fn get_execution_method(&self) -> RuleClosure;
}

/// This is how certain we are of our conclusion.
#[derive(Clone, Debug)]
pub enum ServiceDetectionCertainty {
    /// This is the highest level but still isn't absolute.  We found a version
    /// header or banner somewhere and are trusting that.  Obviously this
    /// could be fake or incorrect
    Advertised,
    /// We are pretty certain our conclusion is correct.
    High,
    /// The conclusion is pretty likely to be correct.
    Medium,
    /// The conclusion is more likely than not to be correct.
    Low,
}

/// One conclusion about a service that could be running on a port.  An attempt
/// at service detection on a port might come up with many conclusions but no
/// one will ever be completely certain.  Each instance has a certainty field
/// telling us how sure we are, ranging from "an educated guess" to "it was
/// announced in a banner."
#[derive(Clone, Debug)]
pub struct ServiceDetectionConclusion {
    /// How certain we are about this conclusion.
    pub certainty: ServiceDetectionCertainty,
    /// A human readable string describing the service.  For example "nginx",
    /// "consul" or "postgres"
    pub service_name: String,
    /// We aren't always able to figure out a version, but when we can it will
    /// be set here.  This human readable string could be an exact version or a
    /// range.
    pub service_version: Option<String>,
}

/// Describes the "loudness" of a rule.  This can be measured in the amount of
/// traffic it generated or in how much this traffic stands out.  Requests
/// likely to trip firewall rules, fire alerts, crash services are also
/// considered "loud".  This isn't used by the system today but still required
/// for all rules.  Eventually it will be possible to configure the planner to
/// only run rules within our desired "loudness"
#[allow(dead_code)]
pub enum RuleLoudness {
    /// This rule is going to make a lot of requests.  These requests might be
    /// extremely strange, generate suspicious errors in logs, or possibly even
    /// crash the service.
    BangingTogetherPotsAndPans,
    /// This rule is making somewhere between 3 and 10 or so requests.  It
    Noisy,
    /// This rule will always make at least one request, but probably no more
    /// than 3 or so.  The requests aren't too different than what a service
    /// expects to see during normal use from a trusted, well behaving
    /// client.
    Standard,
    /// This rule may make at most one network request but it is likely it will
    /// makes none.
    Quiet,
    /// This rule makes no additional network requests.  It relies solely on the
    /// results of previous rule runs.
    Silent,
}
