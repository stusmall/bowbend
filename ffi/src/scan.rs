use ::safer_ffi::prelude::*;
use bowbend_core::start_scan as internal_start_scan;
use futures::StreamExt;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
    config::ConfigBuilder,
    report::Report,
    result::{FfiResult, StatusCodes},
};

#[derive_ReprC]
#[repr(C)]
#[derive(Debug)]
pub struct StreamItem<T> {
    complete: bool,
    item: Option<repr_c::Box<T>>,
}

impl<T> StreamItem<T> {
    fn next(item: T) -> Self {
        Self {
            complete: false,
            item: Some(Box::new(item).into()),
        }
    }

    fn done() -> Self {
        Self {
            complete: true,
            item: None,
        }
    }
}

#[ffi_export]
pub fn free_stream_item(_item: StreamItem<FfiResult<Report>>) {}

#[derive_ReprC]
#[repr(opaque)]
pub struct Scan {
    _runtime: Runtime,
    _handle: JoinHandle<()>,
}

impl Scan {
    fn new(runtime: Runtime, handle: JoinHandle<()>) -> repr_c::Box<Self> {
        Box::new(Scan {
            _runtime: runtime,
            _handle: handle,
        })
        .into()
    }
}

/// Simple method to free a returned scan result.  It just takes ownership then
/// drops it.
#[ffi_export]
pub fn free_scan(_item: FfiResult<Scan>) {}

/// The entry point to kicking off an actual scan.  The `sdk-test-stub` feature
/// is available so that instead of kicking off a scan we dump configs to disk
/// and write fake responses.  This is just here for unit testing SDKs
#[ffi_export]
pub fn start_scan(
    builder: &ConfigBuilder,
    callback: extern "C" fn(StreamItem<FfiResult<Report>>),
) -> FfiResult<Scan> {
    let config = builder.clone();
    let rt = Runtime::new().unwrap();
    let handle = rt.spawn(async move {
        let mut stream = match internal_start_scan(config.into()).await {
            Ok(stream) => stream,
            Err(e) => {
                callback(StreamItem::next(e.into()));
                callback(StreamItem::done());
                return;
            }
        };
        while let Some(internal_report) = stream.next().await {
            let report = Report::from(internal_report);
            let ret = StreamItem::next(FfiResult {
                status_code: StatusCodes::Ok,
                contents: Some(Box::new(report).into()),
            });

            callback(ret);
        }
        callback(StreamItem::done())
    });
    FfiResult {
        status_code: StatusCodes::Ok,
        contents: Some(Scan::new(rt, handle)),
    }
}
