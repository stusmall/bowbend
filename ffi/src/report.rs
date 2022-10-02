use ::safer_ffi::prelude::*;
use bowbend_core::{
    err::PortscanErr,
    icmp::{PingResult as InternalPingResult, PingResultType as InternalPingResultType},
    report::{
        PortReport as InternalPortReport, PortStatus as InternalPortStatus,
        Report as InternalReport, ReportContents as InternalReportContents,
    },
};
use safer_ffi::boxed::Box;

use crate::{
    ip::Ip,
    result::{FfiResult, StatusCodes},
    target::Target,
};
//TODO: We need a way to mark a stream as finished.  It is implicit in the rust
// stream, but not with these reports
#[derive_ReprC]
#[repr(C)]
pub struct Report {
    pub target: Target,
    pub instance: Option<Box<Ip>>,
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
            instance: to_convert.instance.map(|x| Box::new(x.into())),
            contents,
        }
    }
}

#[derive_ReprC]
#[repr(C)]
pub struct ReportContents {
    icmp: Option<Box<PingResult>>,
    // It would be nice if we could make this a hMap<PortNumber, PortStatus>.  Right now
    // safer_ffi doesn't have an FFI friendly Map yet but I bet it will eventually
    ports: safer_ffi::Vec<PortReport>,
}

impl From<InternalReportContents> for ReportContents {
    fn from(to_convert: InternalReportContents) -> Self {
        let ports: safer_ffi::Vec<PortReport> = to_convert
            .ports
            .map(|ports| {
                let x: Vec<PortReport> = ports.into_iter().map(PortReport::from).collect();
                safer_ffi::Vec::from(x)
            })
            .unwrap_or(safer_ffi::Vec::EMPTY);

        let icmp = to_convert.icmp.map(|x| Box::new(x.into()));
        ReportContents { icmp, ports }
    }
}

#[derive_ReprC]
#[repr(C)]
pub struct PortReport {
    pub port: u16,
    pub status: PortStatus,
}

impl From<InternalPortReport> for PortReport {
    fn from(x: InternalPortReport) -> Self {
        PortReport {
            port: x.port,
            status: x.status.into(),
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
