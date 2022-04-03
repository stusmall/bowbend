//! The API of the module is completely unstable.  Structs will be changed,
//! reordered, or removed without compunction.  The only stable APIs are
//! available in the language specific SDKs that depend on this module

#![warn(missing_docs)]
mod builder;
mod result;
mod targets;

use ::safer_ffi::prelude::*;
use portscanner_core::entry_point;

use crate::builder::PortscanBuilder;

/// The entry point to kicking off an actual scan.  The `sdk-test-stub` feature
/// is available so that instead of kicking off a scan we dump configs to disk
/// and write fake responses.  This is just here for unit testing SDKs
#[ffi_export]
pub fn start_scan(builder: &mut PortscanBuilder) {
    let targets: Vec<portscanner_core::target::PortscanTarget> = builder
        .targets
        .to_vec()
        .into_iter()
        .map(|x| x.into())
        .collect();
    #[cfg(feature = "sdk-test-stub ")]
    {
        println!("{:?}", targets);
    }
    #[cfg(not(feature = "sdk-test-stub "))]
    {
        let _ = entry_point(targets, builder.ports.clone(), None);
    }
}

/// The following test function is necessary for the header generation.
#[::safer_ffi::cfg_headers]
#[test]
fn generate_headers() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let root = std::path::Path::new(&env!("CARGO_MANIFEST_DIR"));
    let full_header = root.join("../target/portscanner.h");
    let python_cffi_header = root.join("../target/portscanner_no_includes.h");
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
