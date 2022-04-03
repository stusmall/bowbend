use std::io;

#[derive(Debug)]
pub enum PortscanErr {
    FailedToResolveHostname(io::Error),
}
