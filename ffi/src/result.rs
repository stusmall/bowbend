use ::safer_ffi::prelude::*;

/// This is a poor imitation of the Result enum provided by rust.  If
/// status_code is 0, then it is an OK and contents will be set.  If it is
/// nonzero then the status code will be the error type.
#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct FfiResult<T> {
    pub status_code: StatusCodes,
    pub contents: Option<repr_c::Box<T>>,
}

/// Only useful as part of `FfiResult`.  Tells us if it is Ok or Err and the
/// type of the error
#[derive_ReprC]
#[repr(i8)]
#[derive(Debug)]
pub enum StatusCodes {
    Ok = 0,
    InvalidLength = -1,
}
