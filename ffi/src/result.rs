use ::safer_ffi::prelude::*;
use bowbend_core::err::PortscanErr;

/// This is a poor imitation of the [std::result::Result] enum provided by rust.
/// If `status_code` is 0, then it is an `Ok` and contents will be set.  If it
/// is nonzero then the status code will be the error type.
#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct FfiResult<T> {
    pub status_code: StatusCodes,
    pub contents: Option<repr_c::Box<T>>,
}

/// Only useful as part of `FfiResult`.  Tells us if it is `Ok` or `Err` and the
/// type of the error
#[derive_ReprC]
#[repr(i8)]
#[derive(Debug)]
pub enum StatusCodes {
    Ok = 0,
    /// The buffer provided isn't the length expected.  For example an IPv4
    /// address that isn't 4 bytes long.
    InvalidLength = -1,
    /// The buffer provided doesn't contain valid UTF-8.
    InvalidUTF8 = -2,
    /// The provided hostname failed to resolve.
    FailedToResolveHostname = -3,
    /// The most likely cause of this is trying to use a raw socket when not
    /// root.  This is basically anything besides a full open scan.
    InsufficientPermission = -4,
    /// A range provided is invalid.  One example possible cause is in the
    /// minimum is equal to or greater than the maximum.
    InvalidRange = -5,
    /// We've failed to setup for a portscan for some unknown, internal error.
    UnknownError = -100,
}

impl<T> FfiResult<T> {
    /// A constructor for the [Ok](std::result::Result::Ok) variant of our FFI
    /// friendly [std::result::Result]
    pub fn ok(value: T) -> Self {
        FfiResult {
            status_code: StatusCodes::Ok,
            contents: Some(repr_c::Box::new(value)),
        }
    }

    /// A constructor for the [Err](std::result::Result::Err) variant of our FFI
    /// friendly [std::result::Result]
    pub fn err(status_code: StatusCodes) -> Self {
        FfiResult {
            status_code,
            contents: None,
        }
    }
}

impl<T> From<PortscanErr> for FfiResult<T> {
    fn from(e: PortscanErr) -> Self {
        let status_code = match e {
            PortscanErr::FailedToResolveHostname(_) => StatusCodes::FailedToResolveHostname,
            PortscanErr::InsufficientPermission => StatusCodes::InsufficientPermission,
        };
        FfiResult {
            status_code,
            contents: None,
        }
    }
}
