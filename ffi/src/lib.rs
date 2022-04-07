//! The API of the module is completely unstable.  Structs will be changed,
//! reordered, or removed without compunction.  The only stable APIs are
//! available in the language specific SDKs that depend on this module

#![warn(missing_docs)]
mod builder;
mod result;
mod targets;

use ::safer_ffi::prelude::*;
use bowbend_core::entry_point;
use futures::StreamExt;
use tokio::runtime::Runtime;

use crate::builder::Builder;

/// The entry point to kicking off an actual scan.  The `sdk-test-stub` feature
/// is available so that instead of kicking off a scan we dump configs to disk
/// and write fake responses.  This is just here for unit testing SDKs
#[ffi_export]
pub fn start_scan(builder: &Builder, callback: unsafe extern "C" fn()) {
    // TODO: We need some way to pass along a context of an arbitrary memory block
    // for callback to operate on
    let builder = builder.clone();
    let rt = Runtime::new().unwrap();
    unsafe {
        callback();
    }
    rt.spawn(async move {
        let targets: Vec<bowbend_core::target::Target> = builder
            .targets
            .to_vec()
            .into_iter()
            .map(|x| x.into())
            .collect();
        let mut stream = entry_point(targets, builder.ports, None).await;
        while let Some(report) = stream.next().await {
            println!("Report: {:?}", report);
            unsafe {
                callback();
            }
        }
        println!("After scan");
    });
}

/// The following test function is necessary for the header generation.
#[::safer_ffi::cfg_headers]
#[test]
fn generate_headers() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let root = std::path::Path::new(&env!("CARGO_MANIFEST_DIR"));
    let full_header = root.join("../target/bowbend.h");
    let python_cffi_header = root.join("../target/bowbend_no_includes.h");
    ::safer_ffi::headers::builder()
        .to_file(full_header)?
        .generate()?;

    // cffi needs the includes removed and the header run through the pre-processor
    // before it can parse it.  This just removed the includes, the preprocessor
    // step is elsewhere
    let mut v = Vec::new();
    ::safer_ffi::headers::builder()
        .to_writer(&mut v)
        .with_banner("")
        .generate()?;
    let header_contents = std::str::from_utf8(&v)?.to_owned();
    let trimmed_header: String = header_contents
        .lines()
        .filter(|line| !(line.starts_with("#include") || line.is_empty()))
        .map(|l| l.to_owned() + "\n")
        .collect();
    let mut file = std::fs::File::create(python_cffi_header)?;
    file.write(&trimmed_header.into_bytes())?;

    Ok(())
}
