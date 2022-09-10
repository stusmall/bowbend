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
    InvalidLength = -1,
    InvalidUTF8 = -2,
    FailedToResolveHostname = -3,
    /// The most likely cause of this is trying to use a raw socket when not
    /// root.  This is basically anything besides a full open scan
    InsufficientPermission = -4,
    /// We've failed to setup for a portscan for some unknown, internal error.
    UnknownError = -5,
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
