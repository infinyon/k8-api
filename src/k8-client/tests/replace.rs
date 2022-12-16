#[cfg(feature = "k8")]
#[cfg(not(feature = "k8_stream"))]
mod integration_tests {

    use std::collections::HashMap;

    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tracing::debug;

    use fluvio_future::test_async;
    use k8_client::ClientError;
    use k8_client::K8Client;
    use k8_metadata_client::MetadataClient;
    use k8_types::core::service::ServicePort;
    use k8_types::core::service::{ServiceSpec};
    use k8_types::{InputK8Obj, InputObjectMeta, Spec};

    const SPU_DEFAULT_NAME: &str = "spu";

    fn create_client() -> K8Client {
        K8Client::try_default().expect("cluster not initialized")
    }

    fn new_service() -> InputK8Obj<ServiceSpec> {
        let rng = thread_rng();
        let rname: String = rng
            .sample_iter(&Alphanumeric)
            .map(char::from)
            .take(5)
            .collect();
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
    async fn test_object_replace() -> Result<(), ClientError> {
        let new_item = new_service();
        debug!("creating new service: {:#?}", &new_item);
        let client = create_client();
        let created_item = client
            .create_item::<ServiceSpec>(new_item)
            .await
            .expect("service should be created");

        let initial_version = created_item.metadata.resource_version.clone();
        debug!("client resource_version: {}", initial_version);

        assert!(created_item.metadata.labels.contains_key("app"));

        let mut update_item = created_item.as_update();
        update_item.metadata.labels.clear();

        client
            .replace_item(update_item.clone())
            .await
            .expect("replace");

        let updated_service = client
            .retrieve_item::<ServiceSpec, _>(&update_item.metadata)
            .await
            .expect("retrieval");

        assert!(updated_service.metadata.labels.is_empty());

        // clean up
        let input_metadata: InputObjectMeta = created_item.metadata.into();
        client
            .delete_item::<ServiceSpec, _>(&input_metadata)
            .await
            .expect("delete should work");

        Ok(())
    }
}
