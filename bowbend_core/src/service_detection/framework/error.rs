use std::io;

/// The base error type for rules.  This contains all possible error states.
#[derive(Debug)]
pub enum RuleError {
    /// This is used to propagate up some type of an IO error.  The framework
    /// may decide to attempt a retry on some [`io::Error`] variants depending
    /// on user settings
    IOError(io::Error),
    /// This is some kind of hard failure due to bad internal state, coding
    /// errors or something other unexpected issue.  These errors won't be
    /// tried and all dependent rules will be pruned
    InternalRuleError(Box<dyn std::error::Error + Send + Sync>),
}

impl From<io::Error> for RuleError {
    fn from(e: io::Error) -> Self {
        RuleError::IOError(e)
    }
}
