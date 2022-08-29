// implementation of metadata client do nothing
// it is used for testing where to satisfy metadata contract
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::io::Error as IoError;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use k8_diff::DiffError;
use k8_types::{InputK8Obj, K8List, K8Meta, K8Obj, DeleteStatus, K8Watch, Spec, UpdateK8ObjStatus};
use k8_types::options::DeleteOptions;
use crate::diff::PatchMergeType;

use crate::{ListArg, MetadataClient, MetadataClientError, NameSpace, TokenStreamResult};

#[derive(Debug)]
pub enum DoNothingError {
    IoError(IoError),
    DiffError(DiffError),
    JsonError(serde_json::Error),
    PatchError,
    NotFound,
}

impl From<IoError> for DoNothingError {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl From<serde_json::Error> for DoNothingError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonError(error)
    }
}

impl From<DiffError> for DoNothingError {
    fn from(error: DiffError) -> Self {
        Self::DiffError(error)
    }
}

impl fmt::Display for DoNothingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io: {}", err),
            Self::JsonError(err) => write!(f, "{}", err),
            Self::NotFound => write!(f, "not found"),
            Self::DiffError(err) => write!(f, "{:#?}", err),
            Self::PatchError => write!(f, "patch error"),
        }
    }
}

impl MetadataClientError for DoNothingError {
    fn patch_error() -> Self {
        Self::PatchError
    }

    fn not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }
}

pub struct DoNothingClient();

#[async_trait]
impl MetadataClient for DoNothingClient {
    type MetadataClientError = DoNothingError;

    async fn retrieve_item<S, M>(
        &self,
        _metadata: &M,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        K8Obj<S>: DeserializeOwned,
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<K8Obj<S>, Self::MetadataClientError>
    }

    async fn retrieve_items_with_option<S, N>(
        &self,
        _namespace: N,
        _option: Option<ListArg>,
    ) -> Result<K8List<S>, Self::MetadataClientError>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<K8List<S>, Self::MetadataClientError>
    }

    fn retrieve_items_in_chunks<'a, S, N>(
        self: Arc<Self>,
        _namespace: N,
        _limit: u32,
        _option: Option<ListArg>,
    ) -> BoxStream<'a, K8List<S>>
    where
        S: Spec + 'static,
        N: Into<NameSpace> + Send + Sync + 'static,
    {
        futures_util::stream::empty().boxed()
    }

    async fn delete_item_with_option<S, M>(
        &self,
        _metadata: &M,
        _options: Option<DeleteOptions>,
    ) -> Result<DeleteStatus<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<DeleteStatus<S>, Self::MetadataClientError>
    }

    async fn delete_collection<S, M>(
        &self,
        _namespace: NameSpace,
        _label_selector: Option<&str>,
        _option: Option<DeleteOptions>,
    ) -> Result<DeleteStatus<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<DeleteStatus<S>, Self::MetadataClientError>
    }

    async fn create_item<S>(
        &self,
        _value: InputK8Obj<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        InputK8Obj<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send,
    {
        Err(DoNothingError::NotFound) as Result<K8Obj<S>, Self::MetadataClientError>
    }

    async fn update_status<S>(
        &self,
        _value: &UpdateK8ObjStatus<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        UpdateK8ObjStatus<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send + Sync,
        S::Status: Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<K8Obj<S>, Self::MetadataClientError>
    }

    async fn patch<S, M>(
        &self,
        _metadata: &M,
        _patch: &Value,
        _merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<K8Obj<S>, Self::MetadataClientError>
    }

    async fn patch_status<S, M>(
        &self,
        _metadata: &M,
        _patch: &Value,
        _merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        Err(DoNothingError::NotFound) as Result<K8Obj<S>, Self::MetadataClientError>
    }

    fn watch_stream_since<S, N>(
        &self,
        _namespace: N,
        _resource_version: Option<String>,
    ) -> BoxStream<'_, TokenStreamResult<S, Self::MetadataClientError>>
    where
        K8Watch<S>: DeserializeOwned,
        S: Spec + Send + 'static,
        S::Header: Send + 'static,
        S::Status: Send + 'static,
        N: Into<NameSpace>,
    {
        futures_util::stream::empty().boxed()
    }
}
