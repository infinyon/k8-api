use std::env;
use std::fmt;
use std::io::Error as IoError;

#[cfg(feature = "native")]
use isahc::Error as IsahcError;

#[cfg(feature = "hyper2")]
use hyper::error::Error as HyperError;

use crate::http::header::InvalidHeaderValue;
use crate::http::Error as HttpError;

use k8_config::ConfigError;
use k8_diff::DiffError;

use k8_metadata_client::MetadataClientError;

// For error mapping: see: https://doc.rust-lang.org/nightly/core/convert/trait.From.html

#[derive(Debug)]
pub enum ClientError {
    IoError(IoError),
    EnvError(env::VarError),
    JsonError(serde_json::Error),
    DiffError(DiffError),
    HttpError(HttpError),
    InvalidHttpHeader(InvalidHeaderValue),
    #[cfg(feature = "native")]
    IsahcError(IsahcError),
    #[cfg(feature = "hyper2")]
    HyperError(HyperError),
    K8ConfigError(ConfigError),
    PatchError,
    NotFound,
}

impl From<IoError> for ClientError {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl From<env::VarError> for ClientError {
    fn from(error: env::VarError) -> Self {
        Self::EnvError(error)
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonError(error)
    }
}

impl From<DiffError> for ClientError {
    fn from(error: DiffError) -> Self {
        Self::DiffError(error)
    }
}

#[cfg(feature = "native")]
impl From<IsahcError> for ClientError {
    fn from(error: IsahcError) -> Self {
        Self::IsahcError(error)
    }
}

impl From<HttpError> for ClientError {
    fn from(error: HttpError) -> Self {
        Self::HttpError(error)
    }
}

#[cfg(feature = "hyper2")]
impl From<HyperError> for ClientError {
    fn from(error: HyperError) -> Self {
        Self::HyperError(error)
    }
}

impl From<InvalidHeaderValue> for ClientError {
    fn from(error: InvalidHeaderValue) -> Self {
        Self::InvalidHttpHeader(error)
    }
}

impl From<ConfigError> for ClientError {
    fn from(error: ConfigError) -> Self {
        Self::K8ConfigError(error)
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "{}", err),
            Self::HttpError(err) => write!(f, "{}", err),
            Self::EnvError(err) => write!(f, "{}", err),
            Self::JsonError(err) => write!(f, "{}", err),
            Self::NotFound => write!(f, "not found"),
            Self::DiffError(err) => write!(f, "{:#?}", err),
            Self::PatchError => write!(f, "patch error"),
            Self::K8ConfigError(err) => write!(f, "{}", err),
            Self::InvalidHttpHeader(err) => write!(f, "{}", err),
            #[cfg(feature = "native")]
            Self::IsahcError(err) => write!(f, "{}", err),
            #[cfg(feature = "hyper2")]
            Self::HyperError(err) => write!(f, "{}", err),
        }
    }
}

impl MetadataClientError for ClientError {
    fn patch_error() -> Self {
        Self::PatchError
    }

    fn not_founded(&self) -> bool {
        match self {
            Self::NotFound => true,
            _ => false,
        }
    }
}
