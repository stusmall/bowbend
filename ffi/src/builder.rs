use ::safer_ffi::prelude::*;
use safer_ffi::{boxed::Box as FfiBox, slice::slice_ref};

use crate::targets::PortscanTarget;

#[derive_ReprC]
#[ReprC::opaque]
#[derive(Default)]
pub struct PortscanBuilder {
    pub(crate) targets: Vec<PortscanTarget>,
    pub(crate) ports: Vec<u16>,
}

#[ffi_export]
pub fn new_portscan_builder() -> FfiBox<PortscanBuilder> {
    FfiBox::new(PortscanBuilder::default())
}

#[ffi_export]
pub fn add_target(builder: &mut PortscanBuilder, target: &PortscanTarget) {
    builder.targets.push(target.to_owned());
}

#[ffi_export]
pub fn set_port_list(builder: &mut PortscanBuilder, ports: slice_ref<u16>) {
    builder.ports = ports.to_vec();
}
