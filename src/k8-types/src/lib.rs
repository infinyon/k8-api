mod crd;
mod metadata;
pub mod options;
pub mod store;
#[cfg(feature="core")]
pub mod core;
#[cfg(feature="app")]
pub mod app;
#[cfg(feature="storage")]
pub mod storage;

pub use self::crd::*;
pub use self::metadata::*;
pub use self::spec_def::*;


mod spec_def {

    use std::fmt::Debug;

    use serde::de::DeserializeOwned;
    use serde::Deserialize;
    use serde::Serialize;
    
    use super::Crd;

    pub trait Status:
        Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send + Sync
    {
    }

    pub trait Header:
        Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send + Sync
    {
    }

    /// Kubernetes Spec
    pub trait Spec:
        Sized + Debug + Clone + Default + Serialize + DeserializeOwned + Send + Sync
    {
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
        fn make_same(&mut self, _other: &Self) {}
    }

    #[derive(Deserialize, Serialize, Debug, Default, Clone)]
    pub struct DefaultHeader {}

    impl Header for DefaultHeader {}

}