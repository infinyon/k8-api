use std::env;
use std::fmt;
use std::io::Error as IoError;
use std::error::Error;

use hyper::Error as HyperError;

use k8_config::ConfigError;
use k8_diff::DiffError;
use k8_metadata_client::MetadataClientError;


use crate::http::header::InvalidHeaderValue;
use crate::http::Error as HttpError;
use crate::http::status::StatusCode;

// For error mapping: see: https://doc.rust-lang.org/nightly/core/convert/trait.From.html

#[non_exhaustive]
#[derive(Debug)]
pub enum ClientError {
    IoError(IoError),
    EnvError(env::VarError),
    JsonError(serde_json::Error),
    DiffError(DiffError),
    HttpError(HttpError),
    InvalidHttpHeader(InvalidHeaderValue),
    K8ConfigError(ConfigError),
    PatchError,
    HyperError(HyperError),
    Client(StatusCode),
    Other(String)
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            Self::EnvError(err) => Some(err),
            Self::JsonError(err) => Some(err),
            Self::DiffError(err) => Some(err),
            Self::HttpError(err) => Some(err),
            Self::InvalidHttpHeader(err) => Some(err),
            Self::K8ConfigError(err) => Some(err),
            Self::HyperError(err) => Some(err),
            Self::Client(_) => None,
            Self::PatchError => None,
            Self::Other(_) => None
        }
    }
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


#[cfg(feature = "isahc_rustls")]
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

impl From<StatusCode> for ClientError  {
    fn from(code: StatusCode) -> Self {
        Self::Client(code)
    }
}

impl From<HyperError> for ClientError {
    fn from(error: HyperError) -> Self {
        Self::HyperError(error)
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "{}", err),
            Self::HttpError(err) => write!(f, "{}", err),
            Self::EnvError(err) => write!(f, "{}", err),
            Self::JsonError(err) => write!(f, "{}", err),
            Self::Client(status) => write!(f, "client error: {}",status),
            Self::DiffError(err) => write!(f, "{:#?}", err),
            Self::InvalidHttpHeader(err) => write!(f, "{:#?}", err),
            Self::PatchError => write!(f, "patch error"),
            Self::K8ConfigError(err) => write!(f, "{}", err),
            Self::InvalidHttpHeader(err) => write!(f, "{}", err),
            Self::HyperError(err) => write!(f,"{}",err),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl MetadataClientError for ClientError {
    fn patch_error() -> Self {
        Self::PatchError
    }

    fn not_founded(&self) -> bool {
        match self {
            Self::Client(status) => status == &StatusCode::NOT_FOUND,
            _ => false,
        }
    }
}
