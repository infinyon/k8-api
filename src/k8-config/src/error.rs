use std::io::Error as IoError;
use serde_yaml::Error as YamlError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: IoError,
    },
    #[error("Yaml error: {source}")]
    YamlError {
        #[from]
        source: YamlError,
    },
    #[error("No current kubernetes context")]
    NoCurrentContext,
    #[error("Unknown error: {0}")]
    Other(String),
}
