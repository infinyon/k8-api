use std::env;
use std::io::Error as IoError;
use isahc::Error as IsahcError;
use thiserror::Error;

use crate::http::header::InvalidHeaderValue;
use crate::http::Error as HttpError;
use crate::http::status::StatusCode;

use k8_config::ConfigError;
use k8_diff::DiffError;

use k8_metadata_client::MetadataClientError;

// For error mapping: see: https://doc.rust-lang.org/nightly/core/convert/trait.From.html

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: IoError,
    },
    #[error("Environment error: {source}")]
    EnvError {
        #[from]
        source: env::VarError,
    },
    #[error("JSON error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error("Kubernetes object diff error: {source}")]
    DiffError {
        #[from]
        source: DiffError,
    },
    #[error("HTTP error: {source}")]
    HttpError {
        #[from]
        source: HttpError,
    },
    #[error("Invalid HTTP header: {source}")]
    InvalidHttpHeader {
        #[from]
        source: InvalidHeaderValue,
    },
    #[error("Isahc HTTP error: {source}")]
    IsahcError {
        #[from]
        source: IsahcError,
    },
    #[error("Kubernetes config error: {source}")]
    K8ConfigError {
        #[from]
        source: ConfigError,
    },
    #[error("HTTP client error: {source}")]
    Client {
        #[from]
        source: StatusError,
    },
    #[error("Patch error")]
    PatchError,
}

#[derive(Error, Debug)]
#[error("Status code {status}")]
pub struct StatusError {
    status: StatusCode,
}

impl StatusError {
    pub fn new(status: StatusCode) -> Self {
        Self { status }
    }
}

impl MetadataClientError for ClientError {
    fn patch_error() -> Self {
        Self::PatchError
    }

    fn not_founded(&self) -> bool {
        match self {
            Self::Client { source } => source.status == StatusCode::NOT_FOUND,
            _ => false,
        }
    }
}
