// implementation of metadata client do nothing
// it is used for testing where to satisfy metadata contract
use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use k8_types::{InputK8Obj, K8List, K8Meta, K8Obj, DeleteStatus, K8Watch, Spec, UpdateK8ObjStatus};
use k8_types::options::DeleteOptions;
use crate::diff::PatchMergeType;

use crate::{ListArg, MetadataClient, NameSpace, TokenStreamResult};

pub struct DoNothingClient();

#[async_trait]
impl MetadataClient for DoNothingClient {
    async fn retrieve_item<S, M>(&self, _metadata: &M) -> Result<K8Obj<S>>
    where
        K8Obj<S>: DeserializeOwned,
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        Err(anyhow!("not found"))
    }

    async fn retrieve_items_with_option<S, N>(
        &self,
        _namespace: N,
        _option: Option<ListArg>,
    ) -> Result<K8List<S>>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        Err(anyhow!("not found"))
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
    ) -> Result<DeleteStatus<S>>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        Err(anyhow!("not found"))
    }

    async fn create_item<S>(&self, _value: InputK8Obj<S>) -> Result<K8Obj<S>>
    where
        InputK8Obj<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send,
    {
        Err(anyhow!("not found"))
    }

    async fn update_status<S>(&self, _value: &UpdateK8ObjStatus<S>) -> Result<K8Obj<S>>
    where
        UpdateK8ObjStatus<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send + Sync,
        S::Status: Send + Sync,
    {
        Err(anyhow!("not found"))
    }

    async fn patch<S, M>(
        &self,
        _metadata: &M,
        _patch: &Value,
        _merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        Err(anyhow!("not found"))
    }

    async fn patch_status<S, M>(
        &self,
        _metadata: &M,
        _patch: &Value,
        _merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        Err(anyhow!("not found"))
    }

    fn watch_stream_since<S, N>(
        &self,
        _namespace: N,
        _resource_version: Option<String>,
    ) -> BoxStream<'_, TokenStreamResult<S>>
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
