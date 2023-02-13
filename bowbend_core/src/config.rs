use std::ops::Range;

use crate::target::Target;

/// A [builder pattern](https://en.wikipedia.org/wiki/Builder_pattern) implementation to set all
/// parameters for a scan.
#[derive(Clone)]
pub struct ConfigBuilder {
    pub(crate) targets: Vec<Target>,
    pub(crate) ports: Vec<u16>,
    pub(crate) run_service_detection: bool,
    pub(crate) ping: bool,
    pub(crate) tracing: bool,
    pub(crate) throttle_range: Option<Range<u64>>,
    pub(crate) max_in_flight: u32,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        // This is a fairly important Default implementation.  This is where the default
        // settings for all SDKs comes from.
        Self {
            targets: vec![],
            ports: vec![80],
            run_service_detection: false,
            ping: false,
            tracing: false,
            throttle_range: None,
            max_in_flight: 500_000,
        }
    }
}

impl ConfigBuilder {
    /// Add a target to the list of potential targets held in the builder.
    pub fn add_target(&mut self, target: Target) {
        self.targets.push(target)
    }

    /// This replaces the list of ports to scan on each target.  This doesn't
    /// add to the list; it replaces it.
    pub fn set_port_list(&mut self, ports: Vec<u16>) {
        self.ports = ports;
    }

    /// Set if we should attempt to fingerprint services on open ports.
    pub fn set_run_service_detection(&mut self, run_service_detection: bool) {
        self.run_service_detection = run_service_detection;
    }

    /// Set if we should ping each target before scanning or not.
    pub fn set_ping(&mut self, ping: bool) {
        self.ping = ping;
    }

    /// Enable or disable extremely detailed internal logging.  This is only
    /// useful for internal development.
    pub fn set_tracing(&mut self, tracing: bool) {
        self.tracing = tracing;
    }

    /// Set a range to use when generating random pauses in the scan.  The
    /// values are in milliseconds.
    pub fn set_throttle(&mut self, throttle_range: Range<u64>) {
        self.throttle_range = Some(throttle_range);
    }

    /// Clear any previously set throttle.
    pub fn clear_throttle(&mut self) {
        self.throttle_range = None;
    }

    /// Set the maximum number of in flight tasks for a port scan.  This is
    /// useful for limiting resource utilization.
    pub fn set_max_in_flight(&mut self, max_in_flight: u32) {
        self.max_in_flight = max_in_flight;
    }
}
