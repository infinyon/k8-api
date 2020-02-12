mod client;
mod error;
mod wstream;
mod config;
mod stream;


#[cfg(feature = "k8")]
pub mod fixture;

pub use self::client::K8Client;
pub use self::config::K8HttpClientBuilder;
pub use self::error::ClientError;
pub use k8_config::K8Config;

pub mod metadata {
    pub use k8_metadata_client::*;
}

pub use shared::SharedK8Client;
pub use shared::new_shared;

mod shared {

    use std::sync::Arc;
    use super::K8Config;
    use super::ClientError;
    use super::K8Client;

    pub type  SharedK8Client = Arc<K8Client>;

    pub fn new_shared(config: K8Config) -> Result<SharedK8Client,ClientError> {
        let client= K8Client::new(config)?;
        Ok(Arc::new(client))
    }
}

