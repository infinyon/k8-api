#[cfg(feature = "k8")]
mod integration_tests {

    use std::collections::HashMap;
    use std::time::Duration;

    use anyhow::Result;
    use futures_util::future::join;
    use futures_util::StreamExt;
    use once_cell::sync::Lazy;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use tracing::debug;
    use tracing::trace;

    use fluvio_future::test_async;
    use fluvio_future::timer::sleep;

    use k8_client::K8Client;
    use k8_metadata_client::{ApplyResult, MetadataClient};
    use k8_types::core::service::{ServicePort, ServiceSpec};
    use k8_types::{InputK8Obj, InputObjectMeta, K8Watch, Spec};

    const SPU_DEFAULT_NAME: &str = "spu";
    const PORT: u16 = 9002;
    const ITER: u16 = 10;
    const NS: &str = "default";

    const DELAY: Duration = Duration::from_millis(100);

    fn create_client() -> K8Client {
        K8Client::try_default().expect("cluster not initialized")
    }

    static PREFIX: Lazy<String> = Lazy::new(|| {
        let rng = thread_rng();
        rng.sample_iter(&Alphanumeric)
            .map(char::from)
            .take(5)
            .collect()
    });

    fn new_service(item_id: u16) -> InputK8Obj<ServiceSpec> {
        let name = format!("testservice{}{}", item_id, *PREFIX);
        new_service_with_name(name)
    }

    /// create new service item
    fn new_service_with_name(name: String) -> InputK8Obj<ServiceSpec> {
        let mut labels = HashMap::new();
        labels.insert("app".to_owned(), SPU_DEFAULT_NAME.to_owned());
        let mut selector = HashMap::new();
        selector.insert("app".to_owned(), SPU_DEFAULT_NAME.to_owned());

        let service_spec = ServiceSpec {
            cluster_ip: "None".to_owned(),
            ports: vec![ServicePort {
                port: PORT,
                ..Default::default()
            }],
            selector: Some(selector),
            ..Default::default()
        };

        InputK8Obj {
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
        }
    }

    /// create, update and delete random services
    async fn generate_services_data(client: &K8Client) {
        // wait to allow test to retrieve first in order to version
        sleep(DELAY).await;

        // go thru create, update spec and delete
        for i in 0..ITER {
            let new_item = new_service(i);

            debug!("creating service: {}", i);
            let created_item = client
                .create_item::<ServiceSpec>(new_item)
                .await
                .expect("service should be created");

            sleep(DELAY).await;

            let mut update_item = created_item.as_input();
            update_item.spec = ServiceSpec {
                cluster_ip: "None".to_owned(),
                ports: vec![ServicePort {
                    port: PORT,
                    name: Some("t".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            };

            // apply new changes
            debug!("updating service: {}", i);
            let updates = client.apply(update_item).await.expect("apply");

            let updated_item = match updates {
                ApplyResult::Patched(item) => item,
                _ => {
                    panic!("apply does not result in patch");
                }
            };
            trace!("updated item: {:#?}", updated_item);

            sleep(DELAY).await;

            // update only metadata
            let mut update_item_2 = updated_item.as_input();
            update_item_2.metadata.annotations.insert(
                "test-annotations".to_owned(),
                "test-annotations-value".to_owned(),
            );
            // apply new changes
            debug!("updating service meta: {}", i);
            let updates_2 = client.apply(update_item_2).await.expect("apply");
            trace!("updated item meta: {:#?}", updates_2);

            let updated_item_2 = match updates_2 {
                ApplyResult::Patched(item) => item,
                _ => {
                    panic!("apply does not result in patch");
                }
            };

            debug!("deleting service: {}", i);
            client
                .delete_item::<ServiceSpec, _>(&updated_item_2.metadata)
                .await
                .expect("delete should work");

            sleep(DELAY).await;
        }
    }

    // verify client
    async fn verify_client(client: &K8Client) {
        // there should be only 1 service (kubernetes)
        let services = client
            .retrieve_items::<ServiceSpec, _>(NS)
            .await
            .expect("services");
        // assert_eq!(services.items.len(), 1);

        let version = services.metadata.resource_version.clone();
        debug!("using version: {} ", version);

        let mut service_streams = client.watch_stream_since::<ServiceSpec, _>(NS, Some(version));

        for i in 0..ITER {
            debug!("checking service: {}", i);
            let mut add_events = service_streams
                .next()
                .await
                .expect("events")
                .expect("events");
            assert_eq!(add_events.len(), 1);
            let add_event = add_events.pop().unwrap();
            //  debug!("events:{:#?}",events);
            assert!(matches!(add_event.expect("ok"), K8Watch::ADDED(_)));

            let mut update_events = service_streams
                .next()
                .await
                .expect("events")
                .expect("events");
            trace!("update_events {:?}", update_events);
            assert_eq!(update_events.len(), 1);
            let update_event = update_events.pop().unwrap();
            assert!(matches!(update_event.expect("ok"), K8Watch::MODIFIED(_)));

            let mut update_2_events = service_streams
                .next()
                .await
                .expect("events")
                .expect("events");
            trace!("update_events {:?}", update_2_events);
            assert_eq!(update_2_events.len(), 1);
            let update_2_event = update_2_events.pop().unwrap();
            assert!(matches!(update_2_event.expect("ok"), K8Watch::MODIFIED(_)));

            let mut delete_events = service_streams
                .next()
                .await
                .expect("events")
                .expect("events");

            trace!("delete_events {:?}", delete_events);
            assert_eq!(delete_events.len(), 1);
            let delete_event = delete_events.pop().unwrap();
            assert!(matches!(delete_event.expect("ok"), K8Watch::DELETED(_)));
        }
    }

    #[test_async]
    async fn test_service_changes() -> Result<()> {
        let client = create_client();

        join(generate_services_data(&client), verify_client(&client)).await;

        Ok(())
    }

    /*
    TODO: fix this test

    #[test_async]
    async fn test_service_delete_with_option() -> Result<()> {
        use k8_obj_core::metadata::options::{ DeleteOptions, PropogationPolicy };

        let client = create_client();

        let new_item = new_service_with_name("testservice_delete".to_owned());
      //  new_item.metadata.finalizers = vec!["my-finalizer.example.com".to_owned()];


        let created_item = client
               .create_item::<ServiceSpec>(new_item)
               .await
               .expect("service should be created");



        client
            .delete_item_with_option::<ServiceSpec, _>(&created_item.metadata,Some(DeleteOptions {
                propagation_policy: Some(PropogationPolicy::Foreground),
                ..Default::default()
            }))
            .await
            .expect("delete should work");



        assert!(true);

        Ok(())

    }
    */
}
