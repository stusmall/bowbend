//! This gives us a way to give each language a version of builder pattern over
//! the FFI.  The core of the state is held inside the [Builder] struct.  It
//! uses [`ReprC!`]'s opaque so we can give it member structures that aren't FFI
//! friendly.  We will just pass a reference along to [Builder] along with each
//! method so we can work with the struct members on the rust side of FFI.
use std::ops::Range;

use ::safer_ffi::prelude::*;
use safer_ffi::{boxed::Box as FfiBox, slice::slice_ref};

use crate::{
    result::{FfiResult, StatusCodes},
    target::Target,
};

/// A [builder pattern](https://en.wikipedia.org/wiki/Builder_pattern) implementation to set all
/// parameters for [`start_scan`](crate::scan::start_scan).
#[derive_ReprC]
#[ReprC::opaque]
#[derive(Clone)]
pub struct Builder {
    pub(crate) targets: Vec<Target>,
    pub(crate) ports: Vec<u16>,
    pub(crate) ping: bool,
    pub(crate) tracing: bool,
    pub(crate) throttle_range: Option<Range<u64>>,
    pub(crate) max_in_flight: u32,
}

impl Default for Builder {
    fn default() -> Self {
        // This is a fairly important Default implementation.  This is where the default
        // settings for all portscans for all SDKs comes from.
        Self {
            targets: vec![],
            ports: vec![80],
            ping: false,
            tracing: false,
            throttle_range: None,
            max_in_flight: 500_000,
        }
    }
}

/// Constructor for [Builder]
#[ffi_export]
pub fn new_builder() -> FfiBox<Builder> {
    FfiBox::new(Builder::default())
}

/// Destructor for [Builder].  This just takes ownership and drops it so then we
/// can use the derived drop implementation of the structure.
#[ffi_export]
pub fn free_builder(_input: FfiBox<Builder>) {}

/// Add a target to the list of potential targets held in the builder.
#[ffi_export]
pub fn add_target(builder: &mut Builder, target: &Target) {
    builder.targets.push(target.to_owned());
}

/// This replaces the list of ports to scan on each target.  This doesn't add to
/// the list; it replaces it.
#[ffi_export]
pub fn set_port_list(builder: &mut Builder, ports: slice_ref<u16>) {
    builder.ports = ports.to_vec();
}

/// Set if we should ping each target before scanning or not.
#[ffi_export]
pub fn set_ping(builder: &mut Builder, ping: bool) {
    builder.ping = ping;
}

/// Enable or disable extremely detailed internal logging.  This is only useful
/// for internal development.
#[ffi_export]
pub fn set_tracing(builder: &mut Builder, tracing: bool) {
    builder.tracing = tracing;
}

/// Set a range to use when generating random pauses in the scan.  The values
/// are in milliseconds.
#[ffi_export]
pub fn set_throttle(builder: &mut Builder, min: u64, max: u64) -> FfiResult<()> {
    if min == 0 && max == 0 {
        builder.throttle_range = None;
    }
    if max > min {
        builder.throttle_range = Some(Range {
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
pub fn set_max_in_flight(builder: &mut Builder, max_in_flight: u32) {
    builder.max_in_flight = max_in_flight;
}
