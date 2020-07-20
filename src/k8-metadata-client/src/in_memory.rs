use k8_obj_metadata::Crd;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::io::Error as IoError;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::default::Default;

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use k8_diff::DiffError;
use k8_obj_metadata::InputK8Obj;
use k8_obj_metadata::K8List;
use k8_obj_metadata::K8Meta;
use k8_obj_metadata::K8Obj;
use k8_obj_metadata::K8Status;
use k8_obj_metadata::K8Watch;
use k8_obj_metadata::ObjectMeta;
use k8_obj_metadata::Spec;
use k8_obj_metadata::UpdateK8ObjStatus;
use k8_obj_metadata::StatusEnum;

use crate::ListArg;
use crate::MetadataClient;
use crate::MetadataClientError;
use crate::NameSpace;
use crate::TokenStreamResult;

#[derive(Debug)]
pub enum InMemoryError {
    IoError(IoError),
    DiffError(DiffError),
    JsonError(serde_json::Error),
    LockPoisonError,
    PatchError,
    NotFound,
}

impl From<IoError> for InMemoryError {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl From<serde_json::Error> for InMemoryError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonError(error)
    }
}

impl From<DiffError> for InMemoryError {
    fn from(error: DiffError) -> Self {
        Self::DiffError(error)
    }
}

type ReadPoisonError<'a> = PoisonError<RwLockReadGuard<'a, ItemMap>>;

impl<'a> From<ReadPoisonError<'a>> for InMemoryError {
    fn from(_error: ReadPoisonError) -> Self {
        Self::LockPoisonError
    }
}

type WritePoisonError<'a> = PoisonError<RwLockWriteGuard<'a, ItemMap>>;

impl<'a> From<WritePoisonError<'a>> for InMemoryError {
    fn from(_error: WritePoisonError) -> Self {
        Self::LockPoisonError
    }
}

impl fmt::Display for InMemoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io: {}", err),
            Self::JsonError(err) => write!(f, "{}", err),
            Self::NotFound => write!(f, "not found"),
            Self::DiffError(err) => write!(f, "{:#?}", err),
            Self::PatchError => write!(f, "patch error"),
            Self::LockPoisonError => write!(f, "lock poison error"),
        }
    }
}

impl MetadataClientError for InMemoryError {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ItemKey {
    crd: &'static Crd,
    ns: String,
    name: String,
}

impl ItemKey {
    pub fn new<S>(metadata: &dyn K8Meta) -> Self
    where
        S: Spec
    {

        ItemKey {
            crd: S::metadata(),
            ns: metadata.namespace().to_owned(),
            name: metadata.name().to_owned()
        }
    }
}

type ItemMap = HashMap<ItemKey, Value>;

#[derive(Debug, Default)]
pub struct InMemoryClient {
    store: Arc<RwLock<ItemMap>>,
}

impl InMemoryClient {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MetadataClient for InMemoryClient {
    type MetadataClientError = InMemoryError;

    async fn retrieve_item<S, M>(&self, metadata: &M) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        K8Obj<S>: DeserializeOwned,
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        let store = self.store.read()?;
        let item_key = ItemKey::new::<S>(metadata);
        let item_value = store.get(&item_key).ok_or(InMemoryError::NotFound)?;
        let value: K8Obj<S> = serde_json::from_value(item_value.clone())?;

        Ok(K8Obj {
            api_version: value.api_version,
            kind: value.kind,
            metadata: ObjectMeta {
                name: value.metadata.name().to_owned(),
                namespace: value.metadata.namespace().to_owned(),
                ..Default::default()
            },
            spec: value.spec,
            ..Default::default()
        })
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
        unimplemented!();
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
        unimplemented!();
    }

    async fn delete_item<S, M>(&self, metadata: &M) -> Result<K8Status, Self::MetadataClientError>
    where
        S: Spec,
        M: K8Meta + Send + Sync,
    {
        let mut store = self.store.write()?;
        let item_key = ItemKey::new::<S>(metadata);
        let item_value = store.remove(&item_key).ok_or(InMemoryError::NotFound)?;
        let value: K8Obj<S> = serde_json::from_value(item_value.clone())?;

        Ok(K8Status {
            api_version: value.api_version,
            code: None,
            details: None,
            kind: value.kind,
            message: None,
            reason: None,
            status: StatusEnum::SUCCESS,
        })
    }

    async fn create_item<S>(
        &self,
        value: InputK8Obj<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        InputK8Obj<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send,
    {
        let k8_obj = K8Obj {
            api_version: value.api_version,
            kind: value.kind,
            metadata: ObjectMeta {
                name: value.metadata.name().to_owned(),
                namespace: value.metadata.namespace().to_owned(),
                ..Default::default()
            },
            spec: value.spec,
            ..Default::default()
        };

        let item_key = ItemKey::new::<S>(&value.metadata);
        let item_value = serde_json::to_value(&k8_obj)?;
        let mut store = self.store.write()?;
        store.insert(item_key, item_value);

        Ok(k8_obj)
    }

    async fn update_status<S>(
        &self,
        update_k8_status: &UpdateK8ObjStatus<S>,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        UpdateK8ObjStatus<S>: Serialize + Debug,
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send + Sync,
        S::Status: Send + Sync,
    {
        let mut store = self.store.write()?;
        let item_key = ItemKey::new::<S>(&update_k8_status.metadata);
        let item_value = store.get_mut(&item_key).ok_or(InMemoryError::NotFound)?;

        let mut k8_obj: K8Obj<S> = serde_json::from_value(item_value.clone())?;
        k8_obj.status = update_k8_status.status.clone();
        
        *item_value = serde_json::to_value(&k8_obj)?;
        
        Ok(k8_obj)
    }

    async fn patch_spec<S, M>(
        &self,
        _metadata: &M,
        _patch: &Value,
    ) -> Result<K8Obj<S>, Self::MetadataClientError>
    where
        K8Obj<S>: DeserializeOwned,
        S: Spec + Send,
        M: K8Meta + Display + Send + Sync,
    {
        unimplemented!();
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
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {

    use crate::client::MetadataClient;
    use super::InMemoryClient;
    use super::InMemoryError;

    use std::collections::HashMap;

    use flv_future_aio::test_async;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    use k8_obj_metadata::InputK8Obj;
    use k8_obj_metadata::InputObjectMeta;
    use k8_obj_core::service::ServicePort;
    use k8_obj_core::service::ServiceSpec;
    use k8_obj_core::service::ServiceStatus;
    use k8_obj_core::service::LoadBalancerStatus;
    use k8_obj_core::service::LoadBalancerIngress;
    
    use k8_obj_metadata::Spec;
    use k8_obj_metadata::K8Status;
    use k8_obj_metadata::K8Obj;
    use k8_obj_metadata::StatusEnum;
    use k8_obj_metadata::UpdateK8ObjStatus;

    const SPU_DEFAULT_NAME: &'static str = "spu";

    fn new_service() -> InputK8Obj<ServiceSpec> {
        let rng = thread_rng();
        let rname: String = rng.sample_iter(&Alphanumeric).take(5).collect();
        let name = format!("test{}", rname);

        let mut labels = HashMap::new();
        labels.insert("app".to_owned(), SPU_DEFAULT_NAME.to_owned());
        let mut selector = HashMap::new();
        selector.insert("app".to_owned(), SPU_DEFAULT_NAME.to_owned());

        let service_spec = ServiceSpec {
            cluster_ip: "None".to_owned(),
            ports: vec![ServicePort {
                port: 9092,
                ..Default::default()
            }],
            selector: Some(selector),
            ..Default::default()
        };

        let new_item: InputK8Obj<ServiceSpec> = InputK8Obj {
            api_version: ServiceSpec::api_version(),
            kind: ServiceSpec::kind(),
            metadata: InputObjectMeta {
                name: name.to_lowercase(),
                labels,
                namespace: "default".to_owned(),
                ..Default::default()
            },
            spec: service_spec,
            ..Default::default()
        };

        new_item
    }

    #[test_async]
    async fn test_create_and_delete_service() -> Result<(), InMemoryError> {
        let new_item = new_service();
        
        let client = InMemoryClient::new();
        let item = client.create_item::<ServiceSpec>(new_item)
            .await
            .expect("service should be created");
        
        let k8_status = client
            .delete_item::<ServiceSpec, _>(&item.metadata)
            .await
            .expect("delete should work");

        assert_k8_status_for_item(k8_status, item);
        
        Ok(())
    }

    #[test_async]
    async fn test_create_and_retrieve_service() -> Result<(), InMemoryError> {
        let new_item = new_service();
        
        let client = InMemoryClient::new();
        let item = client.create_item::<ServiceSpec>(new_item)
            .await
            .expect("service should be created");      
        
        let retreived_item = client
            .retrieve_item::<ServiceSpec, _>(&item.metadata)
            .await
            .expect("retreive should work");

        assert_eq!(retreived_item, item);
        
        Ok(())
    }

    // #[test_async]
    // async fn test_create_and_update_service_status() -> Result<(), InMemoryError> {
    //     let new_item = new_service();
        
    //     let client = InMemoryClient::new();
    //     let item = client.create_item::<ServiceSpec>(new_item)
    //         .await
    //         .expect("service should be created");      
        

    //     let new_service_status = ServiceStatus {
    //         load_balancer: LoadBalancerStatus {
    //             ingress: vec![LoadBalancerIngress { hostname: Some("localhost".to_owned()), ip: None } ]
    //         }
    //     };
    //     let update = UpdateK8ObjStatus::new(new_service_status, item.metadata.clone().into());

    //     let updated_item = client
    //         .update_status::<ServiceSpec>(&update)
    //         .await
    //         .expect("update should work");

    //     let retreived_item = client
    //         .retrieve_item::<ServiceSpec, _>(&item.metadata)
    //         .await
    //         .expect("retreive should work");

    //     assert_ne!(updated_item, item);
    //     assert_eq!(retreived_item, updated_item);
        
    //     Ok(())
    // }

    fn assert_k8_status_for_item<S>(k8_status: K8Status, item: K8Obj<S>) where S: Spec {
        assert_eq!(k8_status.status, StatusEnum::SUCCESS);
        assert_eq!(k8_status.api_version, item.api_version);
        assert_eq!(k8_status.kind, item.kind);
    }
}
