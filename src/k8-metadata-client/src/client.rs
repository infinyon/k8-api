use std::fmt::Debug;
use std::fmt::Display;
use std::io::Error as IoError;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::future::ready;
use futures_util::future::FutureExt;
use futures_util::stream::once;
use futures_util::stream::BoxStream;
use futures_util::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use serde_json::Error as SerdeJsonError;
use serde_json::Value;
use tracing::debug;
use tracing::trace;

use k8_diff::{ Changes, Diff, DiffError};
use crate::metadata::{ InputK8Obj, K8List, K8Meta, K8Obj, K8Status, K8Watch, Spec, UpdateK8ObjStatus };
use crate::metadata::options::DeleteOptions;

use crate::{ ApplyResult, DiffSpec };

#[derive(Clone)]
pub enum NameSpace {
    All,
    Named(String),
}

impl NameSpace {
    pub fn is_all(&self) -> bool {
        match self {
            Self::All => true,
            _ => false,
        }
    }

    pub fn named(&self) -> &str {
        match self {
            Self::All => "all",
            Self::Named(name) => &name,
        }
    }
}

impl From<String> for NameSpace {
    fn from(namespace: String) -> Self {
        NameSpace::Named(namespace)
    }
}

impl From<&str> for NameSpace {
    fn from(namespace: &str) -> Self {
        NameSpace::Named(namespace.to_owned())
    }
}

#[derive(Default, Clone)]
pub struct ListArg {
    pub field_selector: Option<String>,
    pub include_uninitialized: Option<bool>,
    pub label_selector: Option<String>,
}

/// trait for metadata client
pub trait MetadataClientError: Debug + Display {
    /// is not founded
    fn not_founded(&self) -> bool;

    // create new patch error
    fn patch_error() -> Self;
}

// For error mapping: see: https://doc.rust-lang.org/nightly/core/convert/trait.From.html

pub type TokenStreamResult<S, E> = Result<Vec<Result<K8Watch<S>, E>>, E>;

pub fn as_token_stream_result<S, E>(events: Vec<K8Watch<S>>) -> TokenStreamResult<S, E>
where
    S: Spec,
    S::Status: Serialize + DeserializeOwned,
    S::Header: Serialize + DeserializeOwned,
{
    Ok(events.into_iter().map(|event| Ok(event)).collect())
}

#[async_trait]
pub trait MetadataClient: Send + Sync {
    type MetadataClientError: MetadataClientError
        + Send
        + Display
        + From<IoError>
        + From<DiffError>
        + From<SerdeJsonError>;

    /// retrieval a single item
    async fn retrieve_item<S, M>(
        &self,
        metadata: &M,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync;

    /// retrieve all items a single chunk
    /// this may cause client to hang if there are too many items
    async fn retrieve_items<S, N>(
        &self,
        namespace: N,
    ) -> Result<K8List<S>, Self::MetadataClientError>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        self.retrieve_items_with_option(namespace, None).await
    }

    async fn retrieve_items_with_option<S, N>(
        &self,
        namespace: N,
        option: Option<ListArg>,
    ) -> Result<K8List<S>, Self::MetadataClientError>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync;

    /// returns stream of items in chunks
    fn retrieve_items_in_chunks<'a, S, N>(
        self: Arc<Self>,
        namespace: N,
        limit: u32,
        option: Option<ListArg>,
    ) -> BoxStream<'a, K8List<S>>
    where
        S: Spec + 'static,
        N: Into<NameSpace> + Send + Sync + 'static;

    async fn delete_item_with_option<S, M>(&self, metadata: &M, option: Option<DeleteOptions>) -> Result<K8Status<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync;


    async fn delete_item<S, M>(&self, metadata: &M) -> Result<K8Status<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync {

        self.delete_item_with_option::<S,M>(metadata, None).await

    }

    /// create new object
    async fn create_item<S>(
        &self,
        value: InputK8Obj<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec;

    /// apply object, this is similar to ```kubectl apply```
    /// for now, this doesn't do any optimization
    /// if object doesn't exist, it will be created
    /// if object exist, it will be patched by using strategic merge diff
    async fn apply<S>(
        &self,
        value: InputK8Obj<S>,
    ) -> Result<ApplyResult<S>, Self::MetadataClientError>
    where
        S: Spec,
        Self::MetadataClientError: From<serde_json::Error> + From<DiffError> + Send,
    {
        debug!("{}: applying '{}' changes", S::label(), value.metadata.name);
        trace!("{}: applying {:#?}", S::label(), value);
        match self.retrieve_item(&value.metadata).await {
            Ok(item) => {
                let mut old_spec: S = item.spec;
                old_spec.make_same(&value.spec);
                // we don't care about status
                let new_spec = serde_json::to_value(DiffSpec::from(value.spec.clone()))?;
                let old_spec = serde_json::to_value(DiffSpec::from(old_spec))?;
                let diff = old_spec.diff(&new_spec)?;
                match diff {
                    Diff::None => {
                        debug!("{}: no diff detected, doing nothing", S::label());
                        Ok(ApplyResult::None)
                    }
                    Diff::Patch(p) => {
                        let json_diff = serde_json::to_value(p)?;
                        debug!("{}: detected diff: old vs. new spec", S::label());
                        trace!("{}: new spec: {:#?}", S::label(), &new_spec);
                        trace!("{}: old spec: {:#?}", S::label(), &old_spec);
                        trace!("{}: new/old diff: {:#?}", S::label(), json_diff);
                        let patch_result = self.patch_spec(&value.metadata, &json_diff).await?;
                        Ok(ApplyResult::Patched(patch_result))
                    }
                    _ => Err(Self::MetadataClientError::patch_error()),
                }
            }
            Err(err) => {
                if err.not_founded() {
                    debug!(
                        "{}: item '{}' not found, creating ...",
                        S::label(),
                        value.metadata.name
                    );
                    let created_item = self.create_item(value.into()).await?;
                    Ok(ApplyResult::Created(created_item))
                } else {
                    Err(err)
                }
            }
        }
    }

    /// update status
    async fn update_status<S>(
        &self,
        value: &UpdateK8ObjStatus<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec;

    /// patch existing with spec
    async fn patch_spec<S, M>(
        &self,
        metadata: &M,
        patch: &Value,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync;

    /// stream items since resource versions
    fn watch_stream_since<S, N>(
        &self,
        namespace: N,
        resource_version: Option<String>,
    ) -> BoxStream<'_, TokenStreamResult<S, Self::MetadataClientError>>
    where
        S: Spec + 'static,
        N: Into<NameSpace>;

    fn watch_stream_now<S>(
        &self,
        ns: String,
    ) -> BoxStream<'_, TokenStreamResult<S, Self::MetadataClientError>>
    where
        S: Spec + 'static,
    {
        let ft_stream = async move {
            let namespace = ns.as_ref();
            match self.retrieve_items_with_option(namespace, None).await {
                Ok(item_now_list) => {
                    let resource_version = item_now_list.metadata.resource_version;

                    let items_watch_stream =
                        self.watch_stream_since(namespace, Some(resource_version));

                    let items_list = item_now_list
                        .items
                        .into_iter()
                        .map(|item| Ok(K8Watch::ADDED(item)))
                        .collect();
                    let list_stream = once(ready(Ok(items_list)));

                    list_stream.chain(items_watch_stream).left_stream()
                    // list_stream
                }
                Err(err) => once(ready(Err(err))).right_stream(),
            }
        };

        ft_stream.flatten_stream().boxed()
    }

    /// Check if the object exists, return true or false.
    async fn exists<S, M>(&self, metadata: &M) -> Result<bool, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        debug!("check if '{}' exists", metadata);
        match self.retrieve_item::<S, M>(metadata).await {
            Ok(_) => Ok(true),
            Err(err) => {
                if err.not_founded() {
                    Ok(false)
                } else {
                    Err(err)
                }
            }
        }
    }
}
