//! This module contains everything we need to describe the results of a
//! portscan.

use crate::{
    err::PortscanErr,
    icmp::PingResult,
    service_detection::framework::ServiceDetectionConclusion,
    target::{Target, TargetInstance},
};

/// A portscan will produce a stream of Reports to notify the caller of
/// what happened. Right now only one is produced per target but in the future
/// we may want to produce multiple.
#[derive(Debug)]
pub struct Report {
    /// The original target as provided by the user
    pub target: Target,
    /// The IP the action was actually performed on.  This is left out
    /// when we aren't able to convert to an instance, for example a hostname
    /// that fails to resolve.
    pub instance: Option<TargetInstance>,
    /// Detailed contents of what happened in the portscan.  We will get a
    /// [`ReportContents`] on a successful run and a [`PortscanErr`] on a
    /// failure.  The remote host being down doesn't count as a failure.
    /// An example of a failure is failing to even resolve the hostname due to
    /// an I/O error.
    pub contents: Result<ReportContents, PortscanErr>,
}

/// The detailed report of the portscan if we got to it.
#[derive(Debug)]
pub struct ReportContents {
    /// The results of pinging the host.
    pub icmp: Option<PingResult>,
    /// This will be none if we never made it to the point of running the
    /// portscan, for example if we pinged and it timed out
    pub ports: Option<Vec<PortReport>>,
}

/// The status of an individual port that was scanned.
#[derive(Debug)]
pub struct PortReport {
    /// The port
    pub port: u16,
    /// If it is open, closed, filtered, etc
    pub status: PortStatus,
    /// The summary of all service detection conclusions, if run.
    pub service_detection_conclusions: Option<Vec<ServiceDetectionConclusion>>,
}

/// The state of the port scanned
#[derive(PartialEq, Debug)]
pub enum PortStatus {
    /// The port is ready to open and establish and connection.  We either fully
    /// established one or
    Open,
    /// The port isn't accepting connections.  In the case of a full TCP scan,
    /// the connection failed.  In the case of a SYN scan, we got a RST back.
    Closed,
}
