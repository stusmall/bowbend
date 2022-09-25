use ::safer_ffi::prelude::*;
use bowbend_core::{entry_point, setup_tracing};
use futures::StreamExt;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{
    builder::Builder,
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
            item: Some(repr_c::Box::new(item)),
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
#[ReprC::opaque]
pub struct Scan {
    _runtime: Runtime,
    _handle: JoinHandle<()>,
}

impl Scan {
    fn new(runtime: Runtime, handle: JoinHandle<()>) -> repr_c::Box<Self> {
        repr_c::Box::new(Scan {
            _runtime: runtime,
            _handle: handle,
        })
    }
}

#[ffi_export]
pub fn free_scan(_item: FfiResult<Scan>) {}

/// The entry point to kicking off an actual scan.  The `sdk-test-stub` feature
/// is available so that instead of kicking off a scan we dump configs to disk
/// and write fake responses.  This is just here for unit testing SDKs
#[ffi_export]
pub fn start_scan(
    builder: &Builder,
    callback: unsafe extern "C" fn(StreamItem<FfiResult<Report>>),
) -> FfiResult<Scan> {
    if builder.tracing {
        setup_tracing();
    }

    let builder = builder.clone(); //TODO: Change arg to take ownership.
    let rt = Runtime::new().unwrap();
    let handle = rt.spawn(async move {
        let targets: Vec<bowbend_core::target::Target> =
            builder.targets.iter().cloned().map(|x| x.into()).collect();
        let mut stream = match entry_point(targets, builder.ports, Some(0..1), builder.ping).await {
            Ok(stream) => stream,
            Err(e) => {
                unsafe {
                    callback(StreamItem::next(e.into()));
                    callback(StreamItem::done());
                }
                return;
            }
        };
        while let Some(internal_report) = stream.next().await {
            let report = Report::from(internal_report);
            let ret = StreamItem::next(FfiResult {
                status_code: StatusCodes::Ok,
                contents: Some(repr_c::Box::new(report)),
            });

            unsafe {
                callback(ret);
            }
        }
        unsafe { callback(StreamItem::done()) }
    });
    FfiResult {
        status_code: StatusCodes::Ok,
        contents: Some(Scan::new(rt, handle)),
    }
}
