//! This covers various TCP scanning strategies.  The user will select one of
//! the following strategies and it will be applied to each port to scan on the
//! host.

pub mod full_open;
pub mod syn_scan;
