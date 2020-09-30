use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;


use async_trait::async_trait;
use futures_util::future::FutureExt;
use futures_util::stream::BoxStream;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde_json;
use serde_json::Value;
use tracing::debug;
use tracing::error;
use tracing::instrument;
use tracing::trace;

use isahc::prelude::*;
use isahc::HttpClient;

use k8_config::K8Config;
use k8_metadata_client::ListArg;
use k8_metadata_client::MetadataClient;
use k8_metadata_client::NameSpace;
use k8_metadata_client::PatchMergeType;
use k8_metadata_client::TokenStreamResult;
use k8_obj_metadata::options::ListOptions;
use k8_obj_metadata::InputK8Obj;
use k8_obj_metadata::K8List;
use k8_obj_metadata::K8Meta;
use k8_obj_metadata::K8Obj;
use k8_obj_metadata::K8Status;
use k8_obj_metadata::K8Watch;
use k8_obj_metadata::Spec;
use k8_obj_metadata::UpdateK8ObjStatus;

use crate::http::header::HeaderValue;
use crate::http::header::ACCEPT;
use crate::http::header::AUTHORIZATION;
use crate::http::header::CONTENT_TYPE;
use crate::http::Uri;
use super::stream::BodyStream;
use super::wstream::WatchStream;
use crate::uri::item_uri;
use crate::uri::items_uri;
use crate::ClientError;
use crate::SharedK8Client;

use super::config::IsahcBuilder;
use list_stream::ListStream;

/// K8 Cluster accessible thru API
#[derive(Debug)]
pub struct K8Client {
    client: HttpClient,
    host: String,
    token: Option<String>,
}

impl K8Client {
    // load using default k8 config
    pub fn default() -> Result<Self, ClientError> {
        let config = K8Config::load()?;
        Self::new(config)
    }

    
    pub fn new(config: K8Config) -> Result<Self, ClientError> {
        let helper = IsahcBuilder::new(config)?;
        let host = helper.host();
        let token = helper.token();
        let client = helper.build()?;
        debug!("using k8 token: {:#?}", token);
        Ok(Self {
            client,
            host,
            token,
        })
    }

    fn hostname(&self) -> &str {
        &self.host
    }

    fn finish_request<B>(&self, request: &mut Request<B>) -> Result<(), ClientError>
    where
        B: Into<Body>,
    {
        if let Some(ref token) = self.token {
            let full_token = format!("Bearer {}", token);
            request
                .headers_mut()
                .insert(AUTHORIZATION, HeaderValue::from_str(&full_token)?);
        }
        Ok(())
    }

    /// handle request. this is async function
    #[instrument(skip(self, request))]
    async fn handle_request<B, T>(&self, mut request: Request<B>) -> Result<T, ClientError>
    where
        T: DeserializeOwned,
        B: Into<Body>,
    {
        self.finish_request(&mut request)?;

        let mut resp = self.client.send_async(request).await?;

        let status = resp.status();
        debug!(status = status.as_u16(), "response status");

        if status.is_success() {
            resp.json().map_err(|err| {
                error!("error decoding raw stream : {}", resp.text().expect("text"));
                err.into()
            })
        } else {
            Err(ClientError::Client(status))
        }
        
    }

    /// return stream of chunks, chunk is a bytes that are stream thru http channel
    #[instrument(
        skip(self, uri),
        fields(uri = &*format!("{}", uri))
    )]
    fn stream_of_chunks<S>(&self, uri: Uri) -> impl Stream<Item = Vec<u8>> + '_
    where
        S: Spec,
        K8Watch<S>: DeserializeOwned,
    {
        debug!("streaming");

        let ft = async move {
            let mut request = match Request::get(uri).body(Body::empty()) {
                Ok(req) => req,
                Err(err) => {
                    error!("error uri err: {}", err);
                    return WatchStream::new(BodyStream::empty());
                }
            };

            if let Err(err) = self.finish_request(&mut request) {
                error!("error finish request: {}", err);
                return WatchStream::new(BodyStream::empty());
            };

            match self.client.send_async(request).await {
                Ok(response) => {
                    trace!("res status: {}", response.status());
                    trace!("res header: {:#?}", response.headers());
                    WatchStream::new(BodyStream::new(response.into_body()))
                }
                Err(err) => {
                    error!("error getting streaming: {}", err);
                    WatchStream::new(BodyStream::empty())
                }
            }
        };

        ft.flatten_stream()
    }

    fn stream<S>(&self, uri: Uri) -> impl Stream<Item = TokenStreamResult<S, ClientError>> + '_
    where
        K8Watch<S>: DeserializeOwned,
        S: Spec + 'static,
        S::Status: 'static,
        S::Header: 'static,
    {
        self.stream_of_chunks(uri).map(move |chunk| {
            trace!(
                "decoding raw stream : {}",
                String::from_utf8_lossy(&chunk).to_string()
            );

            let result: Result<K8Watch<S>, serde_json::Error> = serde_json::from_slice(&chunk)
                .map_err(|err| {
                    error!("parsing error, chunk_len: {}, error: {}", chunk.len(), err);
                    error!(
                        "error raw stream {}",
                        String::from_utf8_lossy(&chunk).to_string()
                    );
                    err
                });
            Ok(vec![match result {
                Ok(obj) => {
                    trace!("de serialized: {:#?}", obj);
                    Ok(obj)
                }
                Err(err) => Err(err.into()),
            }])
        })
    }

    #[instrument(
        name = "retrieve_items"
        skip(self, namespace, options),
        fields(spec = S::label())
    )]
    pub async fn retrieve_items_inner<S, N>(
        &self,
        namespace: N,
        options: Option<ListOptions>,
    ) -> Result<K8List<S>, ClientError>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        let uri = items_uri::<S>(self.hostname(), namespace.into(), options);
        debug!(uri = &*format!("{}", uri), "retrieving items");
        let items = self
            .handle_request(Request::get(uri).body(Body::empty())?)
            .await?;
        trace!("items retrieved: {:#?}", items);
        Ok(items)
    }
}

#[async_trait]
impl MetadataClient for K8Client {
    type MetadataClientError = ClientError;

    /// retrieval a single item
    #[instrument(
        skip(self, metadata),
        fields(spec = S::label()),
    )]
    async fn retrieve_item<S, M>(&self, metadata: &M) -> Result<K8Obj<S>, ClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        let uri = item_uri::<S>(self.hostname(), metadata.name(), metadata.namespace(), None);
        debug!(uri = &*format!("{}", uri), "retrieving item");

        self.handle_request(Request::get(uri).body(Body::empty())?)
            .await
    }

    async fn retrieve_items_with_option<S, N>(
        &self,
        namespace: N,
        option: Option<ListArg>,
    ) -> Result<K8List<S>, ClientError>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        let list_option = option.map(|opt| ListOptions {
            field_selector: opt.field_selector,
            label_selector: opt.label_selector,
            ..Default::default()
        });
        self.retrieve_items_inner(namespace, list_option).await
    }

    fn retrieve_items_in_chunks<'a, S, N>(
        self: Arc<Self>,
        namespace: N,
        limit: u32,
        option: Option<ListArg>,
    ) -> BoxStream<'a, K8List<S>>
    where
        S: Spec + 'static,
        N: Into<NameSpace> + Send + Sync + 'static,
    {
        ListStream::new(namespace.into(), limit, option, self.clone()).boxed()
    }

    #[instrument(
        skip(self, metadata),
        fields(
            spec = S::label(),
            name = metadata.name(),
            namespace = metadata.namespace(),
        )
    )]
    async fn delete_item<S, M>(&self, metadata: &M) -> Result<K8Status, ClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        let uri = item_uri::<S>(self.hostname(), metadata.name(), metadata.namespace(), None);
        debug!(uri = &*format!("{}", uri), "delete item");

        self.handle_request(Request::delete(uri).body(Body::empty())?)
            .await
    }

    /// create new object
    #[instrument(
        skip(self, value),
        fields(
            spec = S::label(),
            name = &*value.metadata.name,
            namespace = &*value.metadata.namespace,
        )
    )]
    async fn create_item<S>(&self, value: InputK8Obj<S>) -> Result<K8Obj<S>, ClientError>
    where
        S: Spec,
    {
        let namespace: NameSpace = value.metadata.namespace.clone().into();
        let uri = items_uri::<S>(self.hostname(), namespace, None);
        debug!("creating '{}'", uri);
        trace!("creating RUST {:#?}", &value);

        let bytes = serde_json::to_vec(&value)?;

        trace!(
            "create raw: {}",
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::post(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes)?;

        self.handle_request(request).await
    }

    /// update status
    #[instrument(
        skip(self, value),
        fields(
            spec = S::label(),
            name = &*value.metadata.name,
            namespace = &*value.metadata.namespace,
        )
    )]
    async fn update_status<S>(&self, value: &UpdateK8ObjStatus<S>) -> Result<K8Obj<S>, ClientError>
    where
        S: Spec,
    {
        let uri = item_uri::<S>(
            self.hostname(),
            &value.metadata.name,
            &value.metadata.namespace,
            Some("/status"),
        );
        debug!(uri = &*format!("{}", uri), "updating status");
        trace!("update: {:#?}", &value);

        let bytes = serde_json::to_vec(&value)?;
        trace!(
            "update raw: {}",
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::put(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes)?;

        self.handle_request(request).await
    }

    /// patch existing with spec
    #[instrument(
        skip(self, metadata, patch),
        fields(
            spec = S::label(),
            name = metadata.name(),
            namespace = metadata.namespace(),
        )
    )]
    async fn patch_spec<S, M>(&self, metadata: &M, patch: &Value) -> Result<K8Obj<S>, ClientError>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        debug!("patching item");
        trace!("patch json value: {:#?}", patch);
        let uri = item_uri::<S>(self.hostname(), metadata.name(), metadata.namespace(), None);
        let merge_type = PatchMergeType::for_spec(S::metadata());

        let bytes = serde_json::to_vec(&patch)?;
        trace!("patch raw: {}", String::from_utf8_lossy(&bytes).to_string());

        let request = Request::patch(uri)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, merge_type.content_type())
            .body(bytes)?;

        self.handle_request(request).await
    }

    /// stream items since resource versions
    fn watch_stream_since<S, N>(
        &self,
        namespace: N,
        resource_version: Option<String>,
    ) -> BoxStream<'_, TokenStreamResult<S, Self::MetadataClientError>>
    where
        S: Spec + 'static,
        S::Status: 'static,
        S::Header: 'static,
        N: Into<NameSpace>,
    {
        let opt = ListOptions {
            watch: Some(true),
            resource_version,
            timeout_seconds: Some(3600),
            ..Default::default()
        };
        let uri = items_uri::<S>(self.hostname(), namespace.into(), Some(opt));
        self.stream(uri).boxed()
    }
}

mod list_stream {

    use std::marker::PhantomData;
    use std::mem::replace;
    use std::mem::transmute;
    use std::pin::Pin;
    use std::task::Context;
    use std::task::Poll;

    use tracing::debug;
    use tracing::error;
    use tracing::trace;

    use futures_util::future::FutureExt;
    use futures_util::future::Future;
    use futures_util::stream::Stream;
    use pin_utils::unsafe_pinned;
    use pin_utils::unsafe_unpinned;

    use super::ClientError;
    use super::K8Client;
    use super::K8List;
    use super::ListArg;
    use super::ListOptions;
    use super::NameSpace;
    use super::SharedK8Client;
    use super::Spec;

    pub struct ListStream<'a, S>
    where
        S: Spec,
    {
        arg: Option<ListArg>,
        limit: u32,
        done: bool,
        namespace: NameSpace,
        client: SharedK8Client,
        inner: Option<Pin<Box<dyn Future<Output = Result<K8List<S>, ClientError>> + Send + 'a>>>,
        data1: PhantomData<S>,
    }

    impl<S> ListStream<'_, S>
    where
        S: Spec,
    {
        pub fn new(
            namespace: NameSpace,
            limit: u32,
            arg: Option<ListArg>,
            client: SharedK8Client,
        ) -> Self {
            Self {
                done: false,
                namespace,
                limit,
                arg,
                client,
                inner: None,
                data1: PhantomData,
            }
        }
    }

    impl<'a, S> Unpin for ListStream<'a, S> where S: Spec {}

    impl<'a, S> ListStream<'a, S>
    where
        S: Spec,
    {
        unsafe_pinned!(
            inner:
                Option<Pin<Box<dyn Future<Output = Result<K8List<S>, ClientError>> + Send + 'a>>>
        );
        unsafe_unpinned!(client: SharedK8Client);

        /// given continuation, generate list option
        fn list_option(&self, continu: Option<String>) -> ListOptions {
            let field_selector = match &self.arg {
                None => None,
                Some(arg) => arg.field_selector.clone(),
            };

            let label_selector = match &self.arg {
                None => None,
                Some(arg) => arg.label_selector.clone(),
            };

            ListOptions {
                limit: Some(self.limit),
                continu,
                field_selector,
                label_selector,
                ..Default::default()
            }
        }
    }

    impl<S> ListStream<'_, S>
    where
        S: Spec + 'static,
    {
        fn set_inner(mut self: Pin<&mut Self>, list_option: Option<ListOptions>) {
            let namespace = self.as_ref().namespace.clone();
            let current_client = &self.as_ref().client;
            // HACK, we transmute the lifetime so that we satisfy borrow checker. should be safe....
            let client: &'_ K8Client =
                unsafe { transmute::<&'_ K8Client, &'_ K8Client>(current_client) };
            self.as_mut()
                .inner()
                .replace(client.retrieve_items_inner(namespace, list_option).boxed());
        }
    }

    impl<S> Stream for ListStream<'_, S>
    where
        S: Spec + 'static,
    {
        type Item = K8List<S>;

        fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            trace!(
                "{}: polling, done: {}, inner none: {}",
                S::label(),
                self.as_ref().done,
                self.as_ref().inner.is_none()
            );

            if self.as_ref().done {
                trace!("{} is done, returning none", S::label());
                return Poll::Ready(None);
            }

            if self.as_ref().inner.is_none() {
                trace!("{} no inner set.", S::label());
                let list_option = self.as_ref().list_option(None);
                self.as_mut().set_inner(Some(list_option));
                trace!("{} set inner, returning pending", S::label());
            }

            trace!("{} polling inner", S::label());
            match self.as_mut().inner().as_pin_mut() {
                Some(fut) => match fut.poll(ctx) {
                    Poll::Pending => {
                        trace!("{} inner was pending, loop continue", S::label());
                        return Poll::Pending;
                    }
                    Poll::Ready(val) => {
                        match val {
                            Ok(list) => {
                                debug!("{} inner returned items: {}", S::label(), list.items.len());
                                // check if we have continue
                                if let Some(_cont) = &list.metadata._continue {
                                    debug!("{}: we got continue: {}", S::label(), _cont);
                                    let list_option =
                                        self.as_ref().list_option(Some(_cont.clone()));
                                    self.set_inner(Some(list_option));
                                    trace!("{}: ready and set inner, returning ready", S::label());
                                    return Poll::Ready(Some(list));
                                } else {
                                    debug!("{} no more continue, marking as done", S::label());
                                    // we are done
                                    let _ = replace(&mut self.as_mut().done, true);
                                    return Poll::Ready(Some(list));
                                }
                            }
                            Err(err) => {
                                error!("{}: error in list stream: {}", S::label(), err);
                                let _ = replace(&mut self.as_mut().done, true);
                                return Poll::Ready(None);
                            }
                        }
                    }
                },
                None => panic!("{} inner should be always set", S::label()),
            }
        }
    }
}
