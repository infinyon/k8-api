use serde_yaml::Error as SerdeYamlError;
use std::io::Error as IoError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(#[from] IoError),
    #[error("Yaml error: {0}")]
    SerdeError(#[from] SerdeYamlError),
    #[error("No active Kubernetes context")]
    NoCurrentContext,
    #[error("Unknown error: {0}")]
    Other(String),
}
