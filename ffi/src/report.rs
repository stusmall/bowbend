use std::collections::HashMap;

use ::safer_ffi::prelude::*;
use bowbend_core::{
    PingResult as InternalPingResult, PingResultType as InternalPingResultType,
    PortReport as InternalPortReport, PortStatus as InternalPortStatus, PortscanErr,
    Report as InternalReport, ReportContents as InternalReportContents,
};
use safer_ffi::boxed::Box as FfiBox;

use crate::{
    ip::Ip,
    result::{FfiResult, StatusCodes},
    service_detection::ServiceDetectionConclusion,
    target::Target,
};

/// A final report for one host.  This is should contain everything the
/// portscanner did with the host and learned about it.  It
#[derive_ReprC]
#[repr(C)]
pub struct Report {
    /// The original target of the portscan.  For IP this field will match
    /// `instance`.  Other target types break up into multiple instances.
    /// There will be one report for each.
    pub target: Target,
    pub instance: Option<FfiBox<Ip>>,
    pub contents: FfiResult<ReportContents>,
}

impl From<InternalReport> for Report {
    fn from(to_convert: InternalReport) -> Self {
        let contents = match to_convert.contents {
            Ok(internal_contents) => FfiResult::<ReportContents>::ok(internal_contents.into()),
            Err(e) => match e {
                PortscanErr::FailedToResolveHostname(_) => {
                    FfiResult::err(StatusCodes::FailedToResolveHostname)
                }
                PortscanErr::InsufficientPermission => {
                    FfiResult::err(StatusCodes::InsufficientPermission)
                }
            },
        };

        Report {
            target: to_convert.target.into(),
            instance: to_convert
                .instance
                .map(|x| Box::<Ip>::new(x.get_ip().into()).into()),
            contents,
        }
    }
}

#[derive_ReprC]
#[repr(opaque)]
pub struct ReportContents {
    icmp: Option<FfiBox<PingResult>>,
    ports: HashMap<u16, PortReport>,
}

#[ffi_export]
pub fn get_icmp_result(report_contents: &ReportContents) -> Option<&PingResult> {
    report_contents.icmp.as_deref()
}

#[ffi_export]
pub fn get_port_report(report_contents: &ReportContents, port: u16) -> Option<&PortReport> {
    report_contents.ports.get(&port)
}

#[ffi_export]
pub fn get_ports(report_contents: &ReportContents) -> safer_ffi::Vec<u16> {
    report_contents
        .ports
        .keys()
        .cloned()
        .collect::<Vec<u16>>()
        .into()
}

#[ffi_export]
pub fn free_port_list(_ports: safer_ffi::Vec<u16>) {}

impl From<InternalReportContents> for ReportContents {
    fn from(to_convert: InternalReportContents) -> Self {
        let ports: HashMap<u16, PortReport> = to_convert
            .ports
            .map(|ports| {
                ports
                    .into_iter()
                    .map(|(port, port_report)| (port, PortReport::from(port_report)))
                    .collect()
            })
            .unwrap_or_default();

        let icmp = to_convert
            .icmp
            .map(|x| Box::<PingResult>::new(x.into()).into());
        ReportContents { icmp, ports }
    }
}

#[derive_ReprC]
#[repr(C)]
pub struct PortReport {
    pub port: u16,
    pub status: PortStatus,
    pub service_detection_conclusions: Option<safer_ffi::Vec<ServiceDetectionConclusion>>,
}

impl From<InternalPortReport> for PortReport {
    fn from(x: InternalPortReport) -> Self {
        PortReport {
            port: x.port,
            status: x.status.into(),
            service_detection_conclusions: x.service_detection_conclusions.map(|conclusions| {
                safer_ffi::Vec::from(
                    conclusions
                        .into_iter()
                        .map(ServiceDetectionConclusion::from)
                        .collect::<Vec<ServiceDetectionConclusion>>(),
                )
            }),
        }
    }
}

#[derive_ReprC]
#[repr(i8)]
pub enum PortStatus {
    Open = 0,
    Closed = 1,
}

impl From<InternalPortStatus> for PortStatus {
    fn from(x: InternalPortStatus) -> Self {
        match x {
            InternalPortStatus::Open => PortStatus::Open,
            InternalPortStatus::Closed => PortStatus::Closed,
        }
    }
}

#[derive_ReprC]
#[repr(i8)]
pub enum PingResultType {
    ReceivedReply = 0,
    IoError = 1,
    Timeout = 2,
}

#[derive_ReprC]
#[repr(C)]
pub struct PingResult {
    result_type: PingResultType,
}

impl From<InternalPingResult> for PingResult {
    fn from(internal: InternalPingResult) -> Self {
        // We can add the timestamps of these events on.  Right now u128 isn't ReprC but
        // once it is, we can easily add them
        match internal.result_type {
            InternalPingResultType::Error(_) => PingResult {
                result_type: PingResultType::IoError,
            },
            InternalPingResultType::Timeout => PingResult {
                result_type: PingResultType::Timeout,
            },
            InternalPingResultType::Reply(_) => PingResult {
                result_type: PingResultType::ReceivedReply,
            },
        }
    }
}
