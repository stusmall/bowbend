//! The API of the module is completely unstable.  Structs will be changed,
//! reordered, or removed without compunction.  The only stable APIs are
//! available in the language specific SDKs that depend on this module

#![warn(missing_docs)]
mod builder;
mod ip;
mod report;
mod result;
mod scan;
mod target;

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
