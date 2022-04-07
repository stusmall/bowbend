use ::safer_ffi::prelude::*;
use safer_ffi::{boxed::Box as FfiBox, slice::slice_ref};

use crate::targets::Target;

#[derive_ReprC]
#[ReprC::opaque]
#[derive(Clone, Default)]
pub struct Builder {
    pub(crate) targets: Vec<Target>,
    pub(crate) ports: Vec<u16>,
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
