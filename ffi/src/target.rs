//! The target for a portscan.
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ::safer_ffi::prelude::*;
use bowbend_core::Target as InternalTarget;
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use safer_ffi::{char_p::char_p_boxed, slice::slice_ref, string::str_ref};

use crate::result::{FfiResult, StatusCodes};

/// This is only used inside `Target` to tag type of the contents.  It's
/// just a workaround for the limitations of enums across the FFI boundary and
/// shouldn't have much use outside `Target`
#[derive_ReprC]
#[repr(u8)]
#[derive(Clone, Debug)]
pub enum TargetType {
    IPv4,
    IPv6,
    IPv4Network,
    IPv6Network,
    Hostname,
}

/// The target of a portscan.  It could be an IP, a network or a hostname.
#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct Target {
    target_type: TargetType,
    contents: safer_ffi::Vec<u8>,
}

impl Clone for Target {
    fn clone(&self) -> Self {
        Target {
            target_type: self.target_type.clone(),
            // Awkward/lazy way to get around safer_ffi::Vec not implementing clone but okay
            contents: safer_ffi::Vec::from(self.contents.to_vec()),
        }
    }
}

impl From<InternalTarget> for Target {
    fn from(to_convert: InternalTarget) -> Self {
        match to_convert {
            InternalTarget::IP(IpAddr::V4(ipv4)) => Target {
                target_type: TargetType::IPv4,
                contents: safer_ffi::Vec::from(ipv4.octets().to_vec()),
            },
            InternalTarget::IP(IpAddr::V6(ipv6)) => Target {
                target_type: TargetType::IPv6,
                contents: safer_ffi::Vec::from(ipv6.octets().to_vec()),
            },
            InternalTarget::Network(IpNet::V4(networkv4)) => {
                let mut addr_vec = networkv4.addr().octets().to_vec();
                addr_vec.push(networkv4.prefix_len());
                Target {
                    target_type: TargetType::IPv4Network,
                    contents: safer_ffi::Vec::from(addr_vec),
                }
            }
            InternalTarget::Network(IpNet::V6(networkv6)) => {
                let mut addr_vec = networkv6.addr().octets().to_vec();
                addr_vec.push(networkv6.prefix_len());
                Target {
                    target_type: TargetType::IPv6Network,
                    contents: safer_ffi::Vec::from(addr_vec),
                }
            }
            InternalTarget::Hostname(hostname) => Target {
                target_type: TargetType::Hostname,
                contents: safer_ffi::Vec::from(hostname.as_bytes().to_vec()),
            },
        }
    }
}

/// Construct a new `Target` containing an IPv4 address.  If the input
/// slice is anything besides 4 bytes then it will return an error result.
#[ffi_export]
pub fn new_ip_v4_address(input: slice_ref<'_, u8>) -> FfiResult<Target> {
    if input.len() == 4 {
        FfiResult {
            status_code: StatusCodes::Ok,
            contents: Some(
                Box::new(Target {
                    target_type: TargetType::IPv4,
                    contents: safer_ffi::Vec::from(input.to_vec()),
                })
                .into(),
            ),
        }
    } else {
        FfiResult {
            status_code: StatusCodes::InvalidLength,
            contents: None,
        }
    }
}

/// Construct a new `Target` containing an IPv6 address.  If the input
/// slice is anything besides 16 bytes then it will return an error result.
#[ffi_export]
pub fn new_ip_v6_address(input: slice_ref<'_, u8>) -> FfiResult<Target> {
    if input.len() == 16 {
        FfiResult {
            status_code: StatusCodes::Ok,
            contents: Some(
                Box::new(Target {
                    target_type: TargetType::IPv6,
                    contents: safer_ffi::Vec::from(input.to_vec()),
                })
                .into(),
            ),
        }
    } else {
        FfiResult {
            status_code: StatusCodes::InvalidLength,
            contents: None,
        }
    }
}

/// Construct a new `Target` containing a CIDR notated IPv4 network.
#[ffi_export]
pub fn new_ip_v4_network(address: slice_ref<'_, u8>, prefix: u8) -> FfiResult<Target> {
    if address.len() == 4 && prefix <= 32 {
        let input =
            safer_ffi::Vec::from(vec![address[0], address[1], address[2], address[3], prefix]);
        FfiResult {
            status_code: StatusCodes::Ok,
            contents: Some(
                Box::new(Target {
                    target_type: TargetType::IPv4Network,
                    contents: input,
                })
                .into(),
            ),
        }
    } else {
        FfiResult {
            status_code: StatusCodes::InvalidLength,
            contents: None,
        }
    }
}

/// Construct a new `Target` containing a CIDR notated IPv6 network.
#[ffi_export]
pub fn new_ip_v6_network(address: slice_ref<'_, u8>, prefix: u8) -> FfiResult<Target> {
    if address.len() == 16 && prefix <= 128 {
        let mut buffer = address.to_vec();
        buffer.push(prefix);
        FfiResult::ok(Target {
            target_type: TargetType::IPv6Network,
            contents: safer_ffi::Vec::from(buffer),
        })
    } else {
        FfiResult::err(StatusCodes::InvalidLength)
    }
}

/// Construct a new `Target` containing a hostname.
#[ffi_export]
pub fn new_hostname(hostname: str_ref<'_>) -> FfiResult<Target> {
    // We are pretty liberal about what we are willing to attempt a DNS look up on.
    // So we aren't really going to do anytime of validation on hostnames
    let bytes = hostname.as_bytes();
    if std::str::from_utf8(bytes).is_ok() {
        FfiResult {
            status_code: StatusCodes::Ok,
            contents: Some(
                Box::new(Target {
                    target_type: TargetType::Hostname,
                    contents: safer_ffi::Vec::from(bytes.to_vec()),
                })
                .into(),
            ),
        }
    } else {
        FfiResult {
            status_code: StatusCodes::InvalidUTF8,
            contents: None,
        }
    }
}

/// Return a string representing the [Target].  This should be used to implement
/// any language specific `to_string` or display methods
#[ffi_export]
pub fn display_target(target: &Target) -> char_p_boxed {
    let internal_target: InternalTarget = target.clone().into();
    let s = format!("{internal_target}");
    let x = s.try_into();
    x.unwrap()
}

/// Free a previously returned [Target].  This accepts a [`FfiResult`] since all
/// constructors return a result and we want to clean it all up.
#[ffi_export]
pub fn free_target(_to_free: FfiResult<Target>) {}

impl From<Target> for InternalTarget {
    fn from(ffi_target: Target) -> Self {
        match ffi_target.target_type {
            TargetType::IPv4 => InternalTarget::IP(IpAddr::V4(Ipv4Addr::new(
                ffi_target.contents[0],
                ffi_target.contents[1],
                ffi_target.contents[2],
                ffi_target.contents[3],
            ))),
            TargetType::IPv6 => {
                let v: [u8; 16] = ffi_target
                    .contents.to_vec()
                    .try_into()
                    .unwrap_or_else(|v: Vec<u8>| panic!("We reached an invalid internal state by having an IPv6 address of only {} bytes after the length check", v.len()));
                InternalTarget::IP(IpAddr::V6(Ipv6Addr::from(v)))
            }
            TargetType::IPv4Network => {
                let address = Ipv4Addr::new(
                    ffi_target.contents[0],
                    ffi_target.contents[1],
                    ffi_target.contents[2],
                    ffi_target.contents[3],
                );
                // The error state here is triggered by prefixes > 32.  We already check that in
                // the constructor so the unwrap here is safe.
                let network = Ipv4Net::new(address, ffi_target.contents[4]).unwrap();
                InternalTarget::Network(IpNet::V4(network))
            }
            TargetType::IPv6Network => {
                let mut contents = ffi_target.contents.to_vec();
                let prefix = contents.pop().unwrap();
                let v: [u8; 16] = contents
                    .try_into()
                    .unwrap_or_else(|v: Vec<u8>| panic!("We reached an invalid internal state by having an IPv6 network address of only {} bytes after the length check", v.len()));
                let address = Ipv6Addr::from(v);
                // The error state here is triggered by prefixes > 128.  We already check that
                // in the constructor so the unwrap here is safe
                InternalTarget::Network(IpNet::V6(Ipv6Net::new(address, prefix).unwrap()))
            }
            TargetType::Hostname => {
                // This unwrap is safe due to the check in the constructor
                let hostname_str = std::str::from_utf8(&ffi_target.contents)
                    .unwrap()
                    .to_owned();
                InternalTarget::Hostname(hostname_str)
            }
        }
    }
}
