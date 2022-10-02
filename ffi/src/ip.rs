//! FFI safe type for passing IPs over the FFI barrier

use std::net::IpAddr;

use ::safer_ffi::prelude::*;

/// An FFI safe representation of an IP, either IPv4 or IPv6.
#[derive_ReprC]
#[repr(C)]
pub struct Ip {
    /// The raw byte representation of the address.
    ip: safer_ffi::Vec<u8>,
}

impl From<IpAddr> for Ip {
    fn from(ip: IpAddr) -> Self {
        let bytes = match ip {
            IpAddr::V4(ipv4) => safer_ffi::Vec::from(ipv4.octets().to_vec()),
            IpAddr::V6(ipv6) => safer_ffi::Vec::from(ipv6.octets().to_vec()),
        };
        Ip { ip: bytes }
    }
}
