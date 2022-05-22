use ::safer_ffi::prelude::*;
use bowbend_core::{
    err::PortscanErr,
    report::{
        PortReport as InternalPortReport, PortStatus as InternalPortStatus,
        Report as InternalReport, ReportContents as InternalReportContents,
    },
};
use safer_ffi::boxed::Box;

use crate::{
    ip::Ip,
    result::{FfiResult, StatusCodes},
    targets::Target,
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
            Ok(internal_contents) => FfiResult::<ReportContents> {
                status_code: StatusCodes::Ok,
                contents: Some(repr_c::Box::new(internal_contents.into())),
            },
            Err(e) => match e {
                PortscanErr::FailedToResolveHostname(_) => FfiResult {
                    status_code: StatusCodes::FailedToResolveHostname,
                    contents: None,
                },
                PortscanErr::NeedsRootPermission => {
                    unimplemented!()
                }
            },
        };
        Report {
            target: to_convert.target.into(),
            instance: None,
            contents,
        }
    }
}

#[derive_ReprC]
#[repr(C)]
pub struct ReportContents {
    //TODO: need to convert PingReport to use SystemTime instead of Instant to pass over FFI
    ports: Option<safer_ffi::Vec<PortReport>>,
}

impl From<InternalReportContents> for ReportContents {
    fn from(to_convert: InternalReportContents) -> Self {
        let ports: Option<safer_ffi::Vec<PortReport>> = to_convert.ports.map(|ports| {
            let x: Vec<PortReport> = ports.into_iter().map(PortReport::from).collect();
            safer_ffi::Vec::from(x)
        });
        ReportContents { ports }
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
