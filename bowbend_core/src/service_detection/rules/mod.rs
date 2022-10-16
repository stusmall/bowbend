//! Module containing all rules build into the system itself.

#![allow(clippy::new_ret_no_self)]
use crate::service_detection::{
    framework::Rule,
    rules::{
        http::{BasicHttpGetProbe, NginxDetectionRule},
        ssl::BasicSSLProbe,
    },
};

pub mod http;
pub mod ssl;

/// Get all rules that currently exist in the system.  This is used to feed into
/// the test planner.  From the starting list it will filter out undesired rules
/// and build an initial test plan.
pub fn get_all_rules() -> Vec<Box<dyn Rule>> {
    // This isn't a great solution.  It will be easy to forget to add new rules to
    // the list. We should create a macro to populate the list
    vec![
        BasicSSLProbe::new(),
        BasicHttpGetProbe::new(),
        NginxDetectionRule::new(),
    ]
}
