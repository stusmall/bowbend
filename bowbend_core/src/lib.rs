#![warn(missing_docs)]
//! This is the cross language core of the portscanner.  All of the real logic
//! and functionality of the project should be contained here.  It's only
//! consumer is the FFI crate which provides a C API to be consumed by each
//! language SDK.  The APIs of this crate are not public and will not be
//! kept stable

use futures::stream;

pub use crate::{
    config::ConfigBuilder,
    err::PortscanErr,
    icmp::{PingResult, PingResultType},
    report::{PortReport, PortStatus, Report, ReportContents},
    scan::start_scan,
    service_detection::framework::{ServiceDetectionCertainty, ServiceDetectionConclusion},
    target::{Target, TargetInstance},
};

mod config;
mod err;
mod icmp;
mod logging;
mod report;
mod scan;
mod service_detection;
mod target;
mod tcp;
pub(crate) mod utils;
