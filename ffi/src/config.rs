//! This gives us a way to give each language a version of builder pattern over
//! the FFI.  The core of the state is held in a type provided by
//! `bowbend_core`. The methods exposed out of this opaque type across FFI are
//! just simple bridges into the core configuration builder type.
use std::ops::Range;

use ::safer_ffi::prelude::*;
use bowbend_core::ConfigBuilder as InternalConfigBuilder;
use safer_ffi::{boxed::Box as FfiBox, slice::slice_ref};

use crate::{
    result::{FfiResult, StatusCodes},
    target::Target,
};

/// A [builder pattern](https://en.wikipedia.org/wiki/Builder_pattern) implementation to set all
/// parameters for a scan
#[derive_ReprC]
#[ReprC::opaque]
#[derive(Clone, Default)]
pub struct ConfigBuilder {
    contents: InternalConfigBuilder,
}

/// Constructor for [`ConfigBuilder`]
#[ffi_export]
pub fn new_builder() -> FfiBox<ConfigBuilder> {
    FfiBox::new(ConfigBuilder::default())
}

/// Destructor for [`ConfigBuilder`].  This just takes ownership and drops it so
/// then we can use the derived drop implementation of the structure.
#[ffi_export]
pub fn free_builder(_input: FfiBox<ConfigBuilder>) {}

/// Add a target to the list of potential targets held in the builder.
#[ffi_export]
pub fn add_target(builder: &mut ConfigBuilder, target: &Target) {
    builder.contents.add_target(target.clone().into())
}

/// This replaces the list of ports to scan on each target.  This doesn't add to
/// the list; it replaces it.
#[ffi_export]
pub fn set_port_list(builder: &mut ConfigBuilder, ports: slice_ref<u16>) {
    builder.contents.set_port_list(ports.to_vec())
}

/// Set if we should attempt to fingerprint services on open ports.
#[ffi_export]
pub fn set_run_service_detection(builder: &mut ConfigBuilder, run_service_detection: bool) {
    builder
        .contents
        .set_run_service_detection(run_service_detection)
}

/// Set if we should ping each target before scanning or not.
#[ffi_export]
pub fn set_ping(builder: &mut ConfigBuilder, ping: bool) {
    builder.contents.set_ping(ping)
}

/// Enable or disable extremely detailed internal logging.  This is only useful
/// for internal development.
#[ffi_export]
pub fn set_tracing(builder: &mut ConfigBuilder, tracing: bool) {
    builder.contents.set_tracing(tracing)
}

/// Set a range to use when generating random pauses in the scan.  The values
/// are in milliseconds.
#[ffi_export]
pub fn set_throttle(builder: &mut ConfigBuilder, min: u64, max: u64) -> FfiResult<()> {
    if min == 0 && max == 0 {
        builder.contents.clear_throttle();
    }
    if max > min {
        builder.contents.set_throttle(Range {
            start: min,
            end: max,
        });
        FfiResult::ok(())
    } else {
        FfiResult::err(StatusCodes::InvalidRange)
    }
}

/// Set the maximum number of in flight tasks for a port scan.  This is useful
/// for limiting resource utilization.
#[ffi_export]
pub fn set_max_in_flight(builder: &mut ConfigBuilder, max_in_flight: u32) {
    builder.contents.set_max_in_flight(max_in_flight)
}

impl From<ConfigBuilder> for InternalConfigBuilder {
    fn from(builder: ConfigBuilder) -> Self {
        builder.contents
    }
}
