#[cfg(feature = "k8")]
#[cfg(not(feature = "k8_stream"))]
mod integration_tests {

    use std::collections::HashMap;

    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tracing::debug;

    use fluvio_future::test_async;
    use k8_client::http::status::StatusCode;
    use k8_client::ClientError;
    use k8_client::K8Client;
    use k8_metadata_client::MetadataClient;
    use k8_obj_core::service::ServicePort;
    use k8_obj_core::service::{LoadBalancerIngress, ServiceSpec};
    use k8_obj_core::metadata::{InputK8Obj,InputObjectMeta,Spec };

    const SPU_DEFAULT_NAME: &'static str = "spu";

    fn create_client() -> K8Client {
        K8Client::default().expect("cluster not initialized")
    }

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
    async fn test_object_conflict() -> Result<(), ClientError> {
        let new_item = new_service();
        debug!("creating new service: {:#?}", &new_item);
        let client = create_client();
        let item = client
            .create_item::<ServiceSpec>(new_item)
            .await
            .expect("service should be created");

        let initial_version = item.metadata.resource_version.clone();
        debug!("client resource_version: {}", initial_version);

        // update status
        let mut new_status = item.status.clone();
        let ingress = LoadBalancerIngress {
            ip: Some("0.0.0.0".to_string()),
            ..Default::default()
        };
        new_status.load_balancer.ingress.push(ingress);

        let mut new_status2 = item.status.clone();
        let ingress = LoadBalancerIngress {
            ip: Some("1.1.1.1".to_string()),
            ..Default::default()
        };
        new_status2.load_balancer.ingress.push(ingress);

        // manually set ip external ip

        let status_update1 = item.as_status_update(new_status);
        let status_update2 = item.as_status_update(new_status2);

        let updated_item = client.update_status(&status_update1).await.expect("update");

        let updated_version = updated_item.metadata.resource_version.clone();

        debug!("updated resource_version: {}", updated_version);

        assert_ne!(updated_version, initial_version);

        // do another update status which leads to conflict.
        let err = client
            .update_status(&status_update2)
            .await
            .expect_err("update");
        match err {
            ClientError::Client(status) => assert_eq!(status, StatusCode::CONFLICT),
            _ => assert!(false),
        }

        // clean up
        let input_metadata: InputObjectMeta = updated_item.metadata.into();
        client
            .delete_item::<ServiceSpec, _>(&input_metadata)
            .await
            .expect("delete should work");

        Ok(())
    }
}
