use ::safer_ffi::prelude::*;
use bowbend_core::{
    ServiceDetectionCertainty as InternalServiceDetectionCertainty,
    ServiceDetectionConclusion as InternalServiceDetectionConclusion,
};

/// One conclusion about a service that could be running on a port.  An attempt
/// at service detection on a port might come up with many conclusions but no
/// one will ever be completely certain.  Each instance has a certainty field
/// telling us how sure we are, ranging from "an educated guess" to "it was
/// announced in a banner."
#[derive_ReprC]
#[repr(C)]
pub struct ServiceDetectionConclusion {
    pub certainty: ServiceDetectionCertainty,
    pub service_name: safer_ffi::String,
    pub service_version: Option<safer_ffi::String>,
}

impl From<InternalServiceDetectionConclusion> for ServiceDetectionConclusion {
    fn from(x: InternalServiceDetectionConclusion) -> Self {
        Self {
            certainty: x.certainty.into(),
            service_name: x.service_name.into(),
            service_version: x.service_version.map(safer_ffi::String::from),
        }
    }
}

/// This is how certain we are of our conclusion.
#[derive_ReprC]
#[repr(i8)]
pub enum ServiceDetectionCertainty {
    /// This is the highest level but still isn't absolute.  We found a version
    /// header or banner somewhere and are trusting that.  Obviously this
    /// could be fake or incorrect
    Advertised = 0,
    /// We are pretty certain our conclusion is correct.
    High = 1,
    /// The conclusion is pretty likely to be correct.
    Medium = 2,
    /// The conclusion is more likely than not to be correct.
    Low = 3,
}

impl From<InternalServiceDetectionCertainty> for ServiceDetectionCertainty {
    fn from(value: InternalServiceDetectionCertainty) -> Self {
        match value {
            InternalServiceDetectionCertainty::Advertised => ServiceDetectionCertainty::Advertised,
            InternalServiceDetectionCertainty::High => ServiceDetectionCertainty::High,
            InternalServiceDetectionCertainty::Medium => ServiceDetectionCertainty::Medium,
            InternalServiceDetectionCertainty::Low => ServiceDetectionCertainty::Low,
        }
    }
}
