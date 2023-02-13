//! This is the Rust SDK for bowbend, the async, stream based, multi-language
//! portscanner.

// We are lucky we are able to do this today but we may not be able to always do
// this.  bowbend_core is an internal, unstable API.  In the future we might
// want to make changes to it to help support other SDKs without breaking the
// Rust SDK's API.  When that day comes, this is where we can do it.  Instead of
// just reexporting bowbend_core types this module can translate.
pub use bowbend_core::*;
