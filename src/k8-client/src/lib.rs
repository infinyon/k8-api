
mod error;
mod cert;
mod uri;
mod list_stream;


#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;

pub use self::error::ClientError;
pub use k8_config::K8Config;


#[cfg(feature = "hyper2")]
mod hyper;
#[cfg(feature = "hyper2")]
pub use crate::hyper::*;

#[cfg(feature = "k8")]
pub mod fixture;

use cert::*;

pub mod metadata {
    pub use k8_metadata_client::*;
}

pub use shared::SharedK8Client;
pub use shared::new_shared;
pub use shared::load_and_share;

mod shared {

    use std::sync::Arc;
    use super::K8Config;
    use super::ClientError;
    use super::K8Client;

    pub type  SharedK8Client = Arc<K8Client>;

    /// create new shared k8 client based on k8 config
    pub fn new_shared(config: K8Config) -> Result<SharedK8Client,ClientError> {
        let client= K8Client::new(config)?;
        Ok(Arc::new(client))
    }

    /// load k8 config and create shared k8 client
    pub fn load_and_share() -> Result<SharedK8Client,ClientError> {
        let config = K8Config::load()?;
        new_shared(config)
    }
}

