use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Buf;
use futures_util::future::FutureExt;
use futures_util::stream::empty;
use futures_util::stream::BoxStream;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;
use futures_util::stream::TryStreamExt;
use http::StatusCode;
use http::header::InvalidHeaderValue;
use hyper::body::aggregate;
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::header::ACCEPT;
use hyper::header::AUTHORIZATION;
use hyper::header::CONTENT_TYPE;
use hyper::Body;
use hyper::Request;
use hyper::Uri;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::debug;
use tracing::error;
use tracing::trace;

use k8_types::{UpdatedK8Obj, MetaStatus};
use k8_config::K8Config;
use k8_types::{InputK8Obj, K8List, K8Meta, K8Obj, DeleteStatus, K8Watch, Spec, UpdateK8ObjStatus};
use k8_types::options::{ListOptions, DeleteOptions};

use crate::uri::{item_uri, items_uri};
use crate::meta_client::{ListArg, MetadataClient, NameSpace, PatchMergeType, TokenStreamResult};

use super::wstream::WatchStream;
use super::{HyperClient, HyperConfigBuilder, ListStream, LogStream};

const SA_TOKEN_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";

/// K8 Cluster accessible thru API
#[derive(Debug)]
pub struct K8Client {
    client: HyperClient,
    host: String,
    token: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionInfo {
    pub major: String,
    pub minor: String,
    pub git_version: String,
    pub git_commit: String,
    pub git_treestate: String,
    pub build_date: String,
    pub go_version: String,
    pub compiler: String,
    pub platform: String,
}

impl K8Client {
    // load using default k8 config
    pub fn try_default() -> Result<Self> {
        let config = K8Config::load()?;
        Self::new(config)
    }

    pub fn new(config: K8Config) -> Result<Self> {
        let helper = HyperConfigBuilder::new(config)?;
        let host = helper.host();
        let token = helper.token()?;
        let client = helper.build()?;
        debug!("using k8 token: {:#?}", token);
        Ok(Self {
            client,
            host,
            token,
        })
    }

    pub async fn server_version(&self) -> Result<VersionInfo> {
        let uri = format!("{}/version", self.host);
        let info = self
            .handle_request(Request::get(uri).body(Body::empty())?)
            .await?;
        trace!("version info retrieved: {:#?}", info);
        Ok(info)
    }

    fn hostname(&self) -> &str {
        &self.host
    }

    fn finish_request<B>(&self, request: &mut Request<B>) -> Result<(), InvalidHeaderValue>
    where
        B: Into<Body>,
    {
        if let Some(ref token) = self.token {
            let full_token = format!("Bearer {token}");
            request
                .headers_mut()
                .insert(AUTHORIZATION, HeaderValue::from_str(&full_token)?);
        }
        Ok(())
    }

    /// Send a request with a Vec<u8> body, and on 401 retry once with a freshly read SA token.
    async fn handle_request_bytes<T>(&self, request: Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        use std::io::Read;

        let method = request.method().clone();
        let uri = request.uri().clone();
        let version = request.version();
        let headers = request.headers().clone();
        let body_buf = request.into_body();

        trace!("request url: {}", uri);

        // first attempt
        let mut req1 = http::Request::builder()
            .method(method.clone())
            .uri(uri.clone())
            .version(version)
            .body(Body::from(body_buf.clone()))?;
        {
            let h = req1.headers_mut();
            for (k, v) in headers.iter() {
                h.insert(k.clone(), v.clone());
            }
        }
        self.finish_request(&mut req1)?;

        let resp1 = self.client.request(req1).await?;
        let status1 = resp1.status();

        let mut r1 = (aggregate(resp1).await?).reader();
        let mut b1 = Vec::new();
        r1.read_to_end(&mut b1)?;

        // success
        if status1.is_success() {
            return serde_json::from_slice(&b1).map_err(|err| {
                error!("json error: {}", err);
                error!("source: {}", String::from_utf8_lossy(&b1));
                err.into()
            });
        }

        // retry once on 401 with a freshly-read token
        if status1 == StatusCode::UNAUTHORIZED {
            if let Ok(fresh) = std::fs::read_to_string(SA_TOKEN_PATH) {
                let mut req2 = http::Request::builder()
                    .method(method)
                    .uri(uri)
                    .version(version)
                    .body(Body::from(body_buf))?;
                {
                    let h = req2.headers_mut();
                    for (k, v) in headers.iter() {
                        h.insert(k.clone(), v.clone());
                    }
                    let bearer = format!("Bearer {}", fresh.trim());
                    h.insert(AUTHORIZATION, HeaderValue::from_str(&bearer)?);
                }

                let resp2 = self.client.request(req2).await?;
                let status2 = resp2.status();

                let mut r2 = (aggregate(resp2).await?).reader();
                let mut b2 = Vec::new();
                r2.read_to_end(&mut b2)?;

                if status2.is_success() {
                    return serde_json::from_slice(&b2).map_err(|err| {
                        error!("json error: {}", err);
                        error!("source: {}", String::from_utf8_lossy(&b2));
                        err.into()
                    });
                } else {
                    trace!(%status2, "error response received");
                    let api_status: MetaStatus = serde_json::from_slice(&b2).map_err(|err| {
                        error!("json error: {}", err);
                        err
                    })?;
                    return Err(api_status.into());
                }
            }
        }

        let api_status: MetaStatus = serde_json::from_slice(&b1).map_err(|err| {
            error!("json error: {}", err);
            err
        })?;
        Err(api_status.into())
    }

    /// handle request. this is async function
    async fn handle_request<T>(&self, mut request: Request<Body>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        use std::io::Read;

        self.finish_request(&mut request)?;

        trace!("request url: {}", request.uri());
        trace!("request body: {:?}", request.body());

        let resp = self.client.request(request).await?;

        let status = resp.status();

        if status.is_success() {
            let mut reader = (aggregate(resp).await?).reader();
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            trace!(%status, "success response: {}", String::from_utf8_lossy(&buffer));
            serde_json::from_slice(&buffer).map_err(|err| {
                error!("json error: {}", err);
                error!("source: {}", String::from_utf8_lossy(&buffer));
                err.into()
            })
        } else {
            trace!(%status, "error response received");
            let mut reader = (aggregate(resp).await?).reader();
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).map_err(|err| {
                error!("unable to read error response: {}", err);
                err
            })?;
            trace!("error response: {}", String::from_utf8_lossy(&buffer));
            let api_status: MetaStatus = serde_json::from_slice(&buffer).map_err(|err| {
                error!("json error: {}", err);
                err
            })?;
            Err(api_status.into())
        }
    }

    /// return stream of chunks, chunk is a bytes that are stream thru http channel
    #[allow(clippy::useless_conversion)]
    fn stream_of_chunks(&self, uri: Uri) -> impl Stream<Item = Bytes> {
        debug!("streaming: {}", uri);

        let request = http::Request::get(uri)
            .body(Body::empty())
            .and_then(|mut req| {
                self.finish_request(&mut req)?;
                Ok(req)
            });

        let http_client = self.client.clone();

        let ft = async move {
            let request = match request {
                Ok(req) => req,
                Err(err) => {
                    error!("error building request: {}", err);
                    return empty().right_stream();
                }
            };

            match http_client.request(request).await {
                Ok(response) => {
                    trace!("res status: {}", response.status());
                    trace!("res header: {:#?}", response.headers());
                    WatchStream::new(response.into_body().map_err(|err| err.into())).left_stream()
                }
                Err(err) => {
                    error!("error getting streaming: {}", err);
                    empty().right_stream()
                }
            }
        };

        ft.flatten_stream()
    }

    /// return get stream of uri
    fn stream<S>(&self, uri: Uri) -> impl Stream<Item = TokenStreamResult<S>> + '_
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

    pub async fn retrieve_items_inner<S, N>(
        &self,
        namespace: N,
        options: Option<ListOptions>,
    ) -> Result<K8List<S>>
    where
        S: Spec,
        N: Into<NameSpace> + Send + Sync,
    {
        let uri = items_uri::<S>(self.hostname(), namespace.into(), options);
        debug!("{}: retrieving items: {}", S::label(), uri);
        let items = self
            .handle_request(Request::get(uri).body(Body::empty())?)
            .await?;
        trace!("items retrieved: {:#?}", items);
        Ok(items)
    }

    /// replace existing object.
    /// object must exist
    pub async fn replace_item<S>(&self, value: UpdatedK8Obj<S>) -> Result<K8Obj<S>>
    where
        S: Spec,
    {
        let metadata = &value.metadata;
        debug!( name = %metadata.name,"replace item");
        trace!("replace {:#?}", value);
        let uri = item_uri::<S>(
            self.hostname(),
            metadata.name(),
            metadata.namespace(),
            None,
            None,
        )?;

        let bytes = serde_json::to_vec(&value)?;

        trace!(
            "replace uri: {}, raw: {}",
            uri,
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::put(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.into())?;

        self.handle_request(request).await
    }

    pub async fn retrieve_log(
        &self,
        namespace: &str,
        pod_name: &str,
        container_name: &str,
    ) -> Result<LogStream> {
        let sub_resource = format!("/log?container={}&follow={}", container_name, false);
        let uri = item_uri::<k8_types::core::pod::PodSpec>(
            self.hostname(),
            pod_name,
            namespace,
            Some(&sub_resource),
            None,
        )?;
        let stream = self.stream_of_chunks(uri);
        Ok(LogStream(Box::pin(stream)))
    }
}

#[async_trait]
impl MetadataClient for K8Client {
    /// retrieval a single item
    async fn retrieve_item<S, M>(&self, metadata: &M) -> Result<Option<K8Obj<S>>>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        let uri = item_uri::<S>(
            self.hostname(),
            metadata.name(),
            metadata.namespace(),
            None,
            None,
        )?;
        debug!("{}: retrieving item: {}", S::label(), uri);

        let result: Result<K8Obj<S>> = self
            .handle_request(Request::get(uri).body(Body::empty())?)
            .await;

        match result {
            Ok(item) => Ok(Some(item)),
            Err(err) => {
                if let Some(MetaStatus {
                    code: Some(code), ..
                }) = err.downcast_ref()
                {
                    if *code == StatusCode::NOT_FOUND.as_u16() {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn retrieve_items_with_option<S, N>(
        &self,
        namespace: N,
        option: Option<ListArg>,
    ) -> Result<K8List<S>>
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
        ListStream::new(namespace.into(), limit, option, self).boxed()
    }

    async fn delete_item_with_option<S, M>(
        &self,
        metadata: &M,
        option: Option<DeleteOptions>,
    ) -> Result<DeleteStatus<S>>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        use k8_types::MetaStatus;

        let uri = item_uri::<S>(
            self.hostname(),
            metadata.name(),
            metadata.namespace(),
            None,
            None,
        )?;
        debug!("{}: delete item on url: {}", S::label(), uri);

        let body = if let Some(option_value) = option {
            let bytes = serde_json::to_vec(&option_value)?;
            trace!("delete raw : {}", String::from_utf8_lossy(&bytes));

            bytes.into()
        } else {
            Body::empty()
        };
        let request = Request::delete(uri)
            .header(ACCEPT, "application/json")
            .body(body)?;
        let values: serde_json::Map<String, serde_json::Value> =
            self.handle_request(request).await?;
        if let Some(kind) = values.get("kind") {
            if kind == "Status" {
                let status: MetaStatus =
                    serde::Deserialize::deserialize(serde_json::Value::Object(values))?;
                Ok(DeleteStatus::Deleted(status))
            } else {
                let status: K8Obj<S> =
                    serde::Deserialize::deserialize(serde_json::Value::Object(values))?;
                Ok(DeleteStatus::ForegroundDelete(status))
            }
        } else {
            Err(anyhow::anyhow!("missing kind: {:#?}", values))
        }
    }

    /// create new object
    async fn create_item<S>(&self, value: InputK8Obj<S>) -> Result<K8Obj<S>>
    where
        S: Spec,
    {
        let namespace: NameSpace = value.metadata.namespace.clone().into();
        let uri = items_uri::<S>(self.hostname(), namespace, None);
        debug!("creating '{}'", uri);
        trace!("creating RUST {:#?}", &value);

        let bytes = serde_json::to_vec(&value)?;

        trace!(
            "create {} raw: {}",
            S::label(),
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::post(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.into())?;

        self.handle_request(request).await
    }

    /// update status
    async fn update_status<S>(&self, value: &UpdateK8ObjStatus<S>) -> Result<K8Obj<S>>
    where
        S: Spec,
    {
        let uri = item_uri::<S>(
            self.hostname(),
            &value.metadata.name,
            &value.metadata.namespace,
            Some("/status"),
            None,
        )?;
        debug!("updating '{}' status - uri: {}", value.metadata.name, uri);
        trace!("update status: {:#?}", &value);

        let bytes = serde_json::to_vec(&value)?;
        trace!(
            "update raw: {}",
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::put(uri)
            .header(CONTENT_TYPE, "application/json")
            .body(bytes.into())?;

        self.handle_request(request).await
    }

    /// patch existing with spec
    async fn patch<S, M>(
        &self,
        metadata: &M,
        patch: &Value,
        merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        debug!(%metadata, "patching");
        trace!("patch json value: {:#?}", patch);
        let uri = item_uri::<S>(
            self.hostname(),
            metadata.name(),
            metadata.namespace(),
            None,
            None,
        )?;

        let bytes = serde_json::to_vec(&patch)?;

        trace!(
            "patch uri: {}, raw: {}",
            uri,
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::patch(uri)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, merge_type.content_type())
            .body(bytes.into())?;

        self.handle_request(request).await
    }

    /// patch status
    async fn patch_status<S, M>(
        &self,
        metadata: &M,
        patch: &Value,
        merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        self.patch_subresource(metadata, String::from("/status"), patch, merge_type)
            .await
    }

    async fn patch_subresource<S, M>(
        &self,
        metadata: &M,
        subresource: String,
        patch: &Value,
        merge_type: PatchMergeType,
    ) -> Result<K8Obj<S>>
    where
        S: Spec,
        M: K8Meta + Display + Send + Sync,
    {
        tracing::info!(%metadata, "patching subresource");
        tracing::info!("patch json value: {:#?}", patch);
        let params = match &merge_type {
            PatchMergeType::Apply(params) => {
                let params = serde_qs::to_string(&params)?;
                Some(params)
            }
            _ => None,
        };
        let uri = item_uri::<S>(
            self.hostname(),
            metadata.name(),
            metadata.namespace(),
            Some(&subresource),
            params.as_deref(),
        )?;

        let bytes = serde_json::to_vec(&patch)?;

        tracing::info!(
            "patch subresource uri: {}, raw: {}",
            uri,
            String::from_utf8_lossy(&bytes).to_string()
        );

        let request = Request::patch(uri)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, merge_type.content_type())
            .body(bytes.into())?;

        self.handle_request(request).await
    }

    /// stream items since resource versions
    fn watch_stream_since<S, N>(
        &self,
        namespace: N,
        resource_version: Option<String>,
    ) -> BoxStream<'_, TokenStreamResult<S>>
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
