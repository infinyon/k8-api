mod client;
mod error;
//mod wstream;
mod config;
mod stream;


#[cfg(feature = "k8")]
pub mod fixture;


pub use self::client::K8Client;
pub use self::config::K8HttpClientBuilder;
pub use self::error::ClientError;

