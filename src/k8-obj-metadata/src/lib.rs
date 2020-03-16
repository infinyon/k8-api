//!
//! # CRD traits
//!
//! Trait for CRD Spec/Status definition
//!
mod crd;
mod metadata;
pub mod options;
pub mod store;

pub use self::crd::*;
pub use self::metadata::*;


use std::fmt::Debug;
use serde::Serialize;
use serde::Deserialize;
use serde::de::DeserializeOwned;



pub trait Status: Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send  + Sync {}

pub trait Header: Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send  + Sync {}

/// Kubernetes Spec
pub trait Spec: Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send  + Sync  {

    type Status: Status;

    type Header: Header;

    /// if true, spec is namespaced
    const NAME_SPACED: bool = true;

    /// return uri for single instance
    fn metadata() -> &'static Crd;

    fn label() -> &'static str {
        Self::metadata().names.kind
    }

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

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct DefaultHeader{}

impl Header for DefaultHeader{}