use ::safer_ffi::prelude::*;
use safer_ffi::{boxed::Box as FfiBox, slice::slice_ref};

use crate::target::Target;

#[derive_ReprC]
#[ReprC::opaque]
#[derive(Clone)]
pub struct Builder {
    pub(crate) targets: Vec<Target>,
    pub(crate) ports: Vec<u16>,
    pub(crate) ping: bool,
    pub(crate) tracing: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            targets: vec![],
            ports: vec![80],
            ping: false,
            tracing: false,
        }
    }
}

#[ffi_export]
pub fn new_builder() -> FfiBox<Builder> {
    FfiBox::new(Builder::default())
}

#[ffi_export]
pub fn add_target(builder: &mut Builder, target: &Target) {
    builder.targets.push(target.to_owned());
}

#[ffi_export]
pub fn set_port_list(builder: &mut Builder, ports: slice_ref<u16>) {
    builder.ports = ports.to_vec();
}

#[ffi_export]
pub fn set_ping(builder: &mut Builder, ping: bool) {
    builder.ping = ping;
}

#[ffi_export]
pub fn set_tracing(builder: &mut Builder, tracing: bool) {
    builder.tracing = tracing;
}
