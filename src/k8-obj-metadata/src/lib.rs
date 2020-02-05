//!
//! # CRD traits
//!
//! Trait for CRD Spec/Status definition
//!
mod crd;
mod metadata;
pub mod options;

pub use self::crd::*;
pub use self::metadata::*;

pub trait Status: Sized{}

/// Kubernetes Spec
pub trait Spec: Sized {

    type Status: Status;

    /// return uri for single instance
    fn metadata() -> &'static Crd;

    fn api_version() -> String {
        let metadata = Self::metadata();
        if metadata.group == "core" {
            return metadata.version.to_owned();
        }
        format!("{}/{}", metadata.group, metadata.version)
    }

    fn kind() -> String {
        Self::metadata().names.kind.to_owned()
    }

    /// in case of applying, we have some fields that are generated
    /// or override.  So need to special logic to reset them so we can do proper comparison
    fn make_same(&mut self,_other: &Self)  {
    }

}
