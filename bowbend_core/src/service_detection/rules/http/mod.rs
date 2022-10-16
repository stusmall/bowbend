//! This module contains all HTTP(S) based service detection rules.
mod basic_http_probe;
mod nginx;

pub use basic_http_probe::{BasicHttpGetProbe, BasicHttpGetProbeResult};
pub use nginx::{NginxDetectionRule, NginxDetectionRuleResult};
