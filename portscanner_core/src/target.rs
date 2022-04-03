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

use crate::{err::PortscanErr, report::PortscanReport};

/// This structure represents an argument into the port scanner itself.  This
/// will be broken down into individual instances almost immediately.  The
/// [`PortscanTargetInstance`] struct are what the internals will actually work
/// on
#[derive(Clone, Eq, Debug, PartialEq, Hash)]
pub enum PortscanTarget {
    /// One IP address.  IPv4 or IPv6
    IP(IpAddr),
    /// One CIDR block
    Network(IpNet),
    /// An individual hostname to be scanned.  This hostname might not conform
    /// to standards.  We won't to be fairly accepting of trying to scan
    /// malformed hostnames
    Hostname(String),
}

impl Display for PortscanTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PortscanTarget::IP(ip) => ip.fmt(f),
            PortscanTarget::Network(network) => network.fmt(f),
            PortscanTarget::Hostname(hostname) => hostname.fmt(f),
        }
    }
}

impl FromStr for PortscanTarget {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(ip_addr) = IpAddr::from_str(s) {
            Ok(PortscanTarget::IP(ip_addr))
        } else if let Ok(inet) = IpNet::from_str(s) {
            Ok(PortscanTarget::Network(inet))
        } else {
            // Can we filter out some more obviously bad host names?  We want to be
            // pretty liberal with what we allow.  Sometimes we might want to
            // scan something that isn't exactly RFC compliant but still
            // resolves
            Ok(PortscanTarget::Hostname(s.to_owned()))
        }
    }
}

impl From<PortscanTargetInstance> for PortscanTarget {
    fn from(instance: PortscanTargetInstance) -> Self {
        match instance {
            PortscanTargetInstance::IP(ip) => PortscanTarget::IP(ip),
            PortscanTargetInstance::Network { network, .. } => PortscanTarget::Network(network),
            PortscanTargetInstance::Hostname { hostname, .. } => PortscanTarget::Hostname(hostname),
        }
    }
}

/// This is an instance of a portscan target.  This presents what we will
/// actually work on.  The initial inputs target may resolve into multiple
/// instances. A hostname may resolve into multiple hostnames, a network will
/// obviously break up into multiple IPs.
#[derive(Clone, Eq, Debug, PartialEq, Hash)]
pub enum PortscanTargetInstance {
    /// The user requested to scan an individual IP.
    IP(IpAddr),
    /// An instance of a split up CIDR block.  When a user wants to scan a CIDR
    /// block we will break it up into a series of
    /// PortscanTargetInstance::Network instance for each IP in the block.
    Network {
        /// The original block requested to be scanned
        network: IpNet,
        /// The individual IP to be scanned
        instance_ip: IpAddr,
    },
    /// An individual instance of a resolved host name.  Any hostname may
    /// resolve into multiple IP addresses when this happens we will have
    /// one PortscanTargetInstance::Hostname for each IP returned by the DNS
    /// query.
    Hostname {
        /// The original hostname used in the DNS lookup
        hostname: String,
        /// The actual IP we are scanning for this hostname
        resolved_ip: IpAddr,
    },
}

impl PortscanTargetInstance {
    /// An instance will always have an IP associated to it.  This just makes it
    /// easy to grab it.
    pub(crate) fn get_ip(&self) -> IpAddr {
        match self {
            PortscanTargetInstance::IP(ip) => *ip,
            PortscanTargetInstance::Network {
                network: _ip,
                instance_ip,
            } => *instance_ip,
            PortscanTargetInstance::Hostname {
                hostname: _,
                resolved_ip,
            } => *resolved_ip,
        }
    }
}

pub(crate) fn targets_to_instance_stream(
    targets: Vec<PortscanTarget>,
) -> (
    impl Stream<Item = PortscanTargetInstance>,
    Vec<PortscanReport>,
) {
    let mut instances = vec![];
    let mut reports = vec![];
    for target in targets {
        match target {
            PortscanTarget::IP(ip) => instances.push(PortscanTargetInstance::IP(ip)),
            PortscanTarget::Network(network) => {
                //TODO: Eventually this might be producing huge numbers of IPs.  We will want
                // to make a method that will break this up into
                // multiple smaller subnets to scan, pick random ones and randomize their
                // content.
                for ip in network.hosts() {
                    instances.push(PortscanTargetInstance::IP(ip))
                }
            }
            PortscanTarget::Hostname(hostname) => match hostname.to_socket_addrs() {
                Ok(ips) => {
                    for socket_adr in ips {
                        let ip = socket_adr.ip();
                        instances.push(PortscanTargetInstance::Hostname {
                            hostname: hostname.clone(),
                            resolved_ip: ip,
                        });
                    }
                }
                Err(e) => {
                    reports.push(PortscanReport {
                        target: PortscanTarget::Hostname(hostname.to_owned()),
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
