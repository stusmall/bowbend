//! This module contains everything we need to describe the targets to hosts to
//! the portscan.
use std::{
    fmt::{Display, Formatter},
    net::{IpAddr, ToSocketAddrs},
    str::FromStr,
};

use futures::{stream, Stream};
use ipnet::IpNet;
use rand::{seq::SliceRandom, thread_rng};
use tracing::instrument;

use crate::{err::PortscanErr, report::Report};

/// This structure represents an argument into the port scanner itself.  This
/// will be broken down into individual instances almost immediately.  The
/// [`TargetInstance`] structure is what the internals will actually work
/// on
#[derive(Clone, Eq, Debug, PartialEq, Hash)]
pub enum Target {
    /// One IP address.  IPv4 or IPv6
    IP(IpAddr),
    /// One CIDR block
    Network(IpNet),
    /// An individual hostname to be scanned.  This hostname might not conform
    /// to standards.  We won't to be fairly accepting of trying to scan
    /// malformed hostnames
    Hostname(String),
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::IP(ip) => ip.fmt(f),
            Target::Network(network) => network.fmt(f),
            Target::Hostname(hostname) => hostname.fmt(f),
        }
    }
}

impl FromStr for Target {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(ip_addr) = IpAddr::from_str(s) {
            Ok(Target::IP(ip_addr))
        } else if let Ok(inet) = IpNet::from_str(s) {
            Ok(Target::Network(inet))
        } else {
            // Can we filter out some more obviously bad host names?  We want to be
            // pretty liberal with what we allow.  Sometimes we might want to
            // scan something that isn't exactly RFC compliant but still
            // resolves
            Ok(Target::Hostname(s.to_owned()))
        }
    }
}

impl From<TargetInstance> for Target {
    fn from(instance: TargetInstance) -> Self {
        match instance {
            TargetInstance::IP(ip) => Target::IP(ip),
            TargetInstance::Network { network, .. } => Target::Network(network),
            TargetInstance::Hostname { hostname, .. } => Target::Hostname(hostname),
        }
    }
}

/// This is an instance of a portscan target.  This presents what we will
/// actually work on.  The initial inputs target may resolve into multiple
/// instances. A hostname may resolve into multiple hostnames, a network will
/// obviously break up into multiple IPs.
#[derive(Clone, Eq, Debug, PartialEq, Hash)]
pub enum TargetInstance {
    /// The user requested to scan an individual IP.
    IP(IpAddr),
    /// An instance of a split up CIDR block.  When a user wants to scan a CIDR
    /// block we will break it up into a series of Network instances.  One
    /// for each IP in the block.
    Network {
        /// The original block requested to be scanned
        network: IpNet,
        /// The individual IP to be scanned
        instance_ip: IpAddr,
    },
    /// An individual instance of a resolved host name.  Any hostname may
    /// resolve into multiple IP addresses when this happens we will have
    /// one `Hostname` for each IP returned by the DNS query.
    Hostname {
        /// The original hostname used in the DNS lookup
        hostname: String,
        /// The actual IP we are scanning for this hostname
        resolved_ip: IpAddr,
    },
}

impl TargetInstance {
    /// An instance will always have an IP associated to it.  This just makes it
    /// easy to grab it.
    pub(crate) fn get_ip(&self) -> IpAddr {
        match self {
            TargetInstance::IP(ip) => *ip,
            TargetInstance::Network {
                network: _ip,
                instance_ip,
            } => *instance_ip,
            TargetInstance::Hostname {
                hostname: _,
                resolved_ip,
            } => *resolved_ip,
        }
    }
}

#[instrument(level = "trace")]
pub(crate) fn targets_to_instance_stream(
    targets: Vec<Target>,
) -> (impl Stream<Item = TargetInstance>, Vec<Report>) {
    let mut instances = vec![];
    let mut reports = vec![];
    for target in targets {
        match target {
            Target::IP(ip) => instances.push(TargetInstance::IP(ip)),
            Target::Network(network) => {
                //TODO: Eventually this might be producing huge numbers of IPs.  We will want
                // to make a method that will break this up into
                // multiple smaller subnets to scan, pick random ones and randomize their
                // content.
                for ip in network.hosts() {
                    instances.push(TargetInstance::IP(ip))
                }
            }
            Target::Hostname(hostname) => match hostname.to_socket_addrs() {
                Ok(ips) => {
                    for socket_adr in ips {
                        let ip = socket_adr.ip();
                        instances.push(TargetInstance::Hostname {
                            hostname: hostname.clone(),
                            resolved_ip: ip,
                        });
                    }
                }
                Err(e) => {
                    reports.push(Report {
                        target: Target::Hostname(hostname.to_owned()),
                        instance: None,
                        contents: Err(PortscanErr::FailedToResolveHostname(e)),
                    });
                }
            },
        }
    }
    instances.shuffle(&mut thread_rng());
    (stream::iter(instances), reports)
}
