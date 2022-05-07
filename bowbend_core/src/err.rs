//! Module to place any error handling related code
use std::io;


/// This is an issue during the set up of the portscan.
pub enum SetupErr {
    /// We are trying to use some type of action that requires root access, most likely the use of
    /// a raw socket.  Examples of scans that require that are ICMP and SYN scans.
    NeedsRootPermission,
}


/// An error during the process of a portscan.  This covers all possible errors
/// at any point, so not all variants make sense in all cases
#[derive(Debug)]
pub enum PortscanErr {
    /// The hostname target wouldn't resolve
    FailedToResolveHostname(io::Error),
}
