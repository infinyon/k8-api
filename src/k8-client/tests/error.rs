#[cfg(feature = "k8")]
mod integration_tests {

    use std::collections::HashMap;
    use std::time::Duration;

    use anyhow::Result;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tracing::debug;

    use fluvio_future::test_async;
    use fluvio_future::timer::sleep;
    use k8_client::http::status::StatusCode;
    use k8_client::K8Client;
    use k8_metadata_client::MetadataClient;
    use k8_types::core::service::{LoadBalancerType, ServicePort};
    use k8_types::core::service::{LoadBalancerIngress, ServiceSpec};
    use k8_types::{InputK8Obj, InputObjectMeta, Spec, MetaStatus};

    const SPU_DEFAULT_NAME: &str = "spu";
    const DELAY: Duration = Duration::from_millis(100);

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
            ports: vec![ServicePort {
                port: 9092,
                ..Default::default()
            }],
            selector: Some(selector),
            r#type: Some(LoadBalancerType::LoadBalancer),
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
    async fn test_object_conflict() -> Result<()> {
        let new_item = new_service();
        debug!("creating new service: {:#?}", &new_item);
        let client = create_client();
        let created_item = client
            .create_item::<ServiceSpec>(new_item)
            .await
            .expect("service should be created");

        sleep(DELAY).await;

        let item = client
            .retrieve_item::<ServiceSpec, _>(&created_item.metadata)
            .await
            .expect("retrieval")
            .expect("service should exist");

        let initial_version = item.metadata.resource_version.clone();
        debug!("client resource_version: {}", initial_version);

        // update status
        let mut new_status = item.status.clone();
        let ingress = LoadBalancerIngress {
            ip: Some("0.0.0.0".to_string()),
            ip_mode: Some("VIP".to_string()),
            ..Default::default()
        };
        new_status.load_balancer.ingress.push(ingress);

        let mut new_status2 = item.status.clone();
        let ingress = LoadBalancerIngress {
            ip: Some("1.1.1.1".to_string()),
            ip_mode: Some("Proxy".to_string()),
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
        if let Some(status) = err.downcast_ref::<MetaStatus>() {
            assert_eq!(status.code, Some(StatusCode::CONFLICT.as_u16()))
        } else {
            panic!("expecting conflict error");
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
