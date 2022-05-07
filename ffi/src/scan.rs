use std::time::Duration;
use ::safer_ffi::prelude::*;
use bowbend_core::{entry_point, setup_tracing};
use futures::StreamExt;
use tokio::runtime::Runtime;

use crate::builder::Builder;
use crate::report::Report;
use crate::result::{FfiResult, StatusCodes};

/// The entry point to kicking off an actual scan.  The `sdk-test-stub` feature
/// is available so that instead of kicking off a scan we dump configs to disk
/// and write fake responses.  This is just here for unit testing SDKs
#[ffi_export]
pub fn start_scan(builder: &Builder, callback: unsafe extern "C" fn(Report)) {
    // TODO: We need some way to pass along a context of an arbitrary memory block
    // for callback to operate on

    if builder.tracing {
        setup_tracing();
    }

    let builder = builder.clone();
    let rt = Runtime::new().unwrap();
    println!("RUST: starting scan");
    rt.spawn(async move {
        println!("RUST: in rt spawn");
        let targets: Vec<bowbend_core::target::Target> = builder
            .targets
            .to_vec()
            .into_iter()
            .map(|x| x.into())
            .collect();
        println!("RUST: Created targets {:?}", targets);
        let mut stream = entry_point(targets, builder.ports, None, builder.ping).await;
        println!("RUST: we have a stream");
        while let Some(internal_report) = stream.next().await {
            println!("Report: {:?}", internal_report);
            let report = Report::from(internal_report);
            unsafe {
                callback(report);
            }
        }
        println!("RUST: after scan");
        //TODO: remove
        let r = Report{
            target: Default::default(),
            instance: None,
            contents: FfiResult { status_code: StatusCodes::Ok, contents: None }
        };
        unsafe {
            println!("RUST: callback");
            callback(r);
        }
        println!("RUST: end of async task");
    });
    std::thread::sleep(Duration::from_secs(5));
    println!("RUST: exiting method");
}