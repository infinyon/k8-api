#[cfg(feature = "k8")]
mod integration_tests {

    use std::collections::HashMap;
    use std::time::Duration;

    use tracing::debug;
    use tracing::trace;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use once_cell::sync::Lazy;
    use futures_util::future::join;
    use futures_util::StreamExt;

    use fluvio_future::test_async;
    use fluvio_future::timer::sleep;

    use k8_client::{ K8Client, ClientError} ;
    use k8_metadata_client::{ MetadataClient, ApplyResult};
    use k8_obj_core::service::ServicePort;
    use k8_obj_core::service::ServiceSpec;
    use k8_obj_metadata::{ InputK8Obj, InputObjectMeta, Spec, K8Watch};

    const SPU_DEFAULT_NAME: &'static str = "spu";
    const PORT: u16 = 9002;
    const ITER: u16 = 10;
    const NS: &str = "default";
  
    const DELAY: Duration = Duration::from_millis(50);

    fn create_client() -> K8Client {
        K8Client::default().expect("cluster not initialized")
    }

    static PREFIX: Lazy<String> = Lazy::new(|| {
        let rng = thread_rng();
        rng.sample_iter(&Alphanumeric).take(5).collect()
    });

   
    /// create new service item
    fn new_service(item_id: u16) -> InputK8Obj<ServiceSpec> {
        
        let name = format!("testservice{}{}",item_id,*PREFIX);

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

        let new_item = InputK8Obj {
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

    /// create, update and delete random services
    async fn generate_services_data(client: &K8Client) {

        // wait to allow test to retrieve first in order to version
        sleep(DELAY).await;

        // go thru create, update spec and delete
        for i in 0..ITER {
        
            let new_item = new_service(i);

            debug!("creating service: {}",i);
            let created_item = client
                .create_item::<ServiceSpec>(new_item)
                .await
                .expect("service should be created");

            sleep(DELAY).await;

            let mut update_item = created_item.as_input();
            update_item. spec = ServiceSpec {
                cluster_ip: "None".to_owned(),
                ports: vec![ServicePort {
                    port: PORT,
                    name: Some("t".to_owned()),
                    ..Default::default()
                }],
                ..Default::default()
            };

            trace!("updated item: {:#?}",update_item);
            // apply new changes
            debug!("updating service: {}",i);
            let updates = client.apply(update_item).await.expect("apply");

            let updated_item = match updates {
                ApplyResult::Patched(item) => item,
                _ =>  {
                    assert!(false,"apply does not result in patche");
                    panic!();
                }
            };

            sleep(DELAY).await;

            debug!("deleting service: {}",i);
            client
            .delete_item::<ServiceSpec, _>(&updated_item.metadata)
            .await
            .expect("delete should work");

            sleep(DELAY).await;
        }
    }

    // verify client
    async fn verify_client(client: &K8Client) {

        // there should be only 1 service (kubernetes)
        let services = client.retrieve_items::<ServiceSpec, _>(NS).await.expect("services");
        assert_eq!(services.items.len(),1);

    
        let version = services.metadata.resource_version.clone();
        debug!("using version: {} ",version);

        let mut service_streams = client.watch_stream_since::<ServiceSpec,_>(NS, Some(version));

        for i in 0..ITER  {
            
            debug!("checking service: {}",i);
            let mut add_events = service_streams.next().await.expect("events").expect("events");
            assert_eq!(add_events.len(),1);
            let add_event = add_events.pop().unwrap();
             //  debug!("events:{:#?}",events);
            assert!(matches!(add_event.expect("ok"),K8Watch::ADDED(_)));
            
            let mut update_events = service_streams.next().await.expect("events").expect("events");
            let update_event = update_events.pop().unwrap();
            assert!(matches!(update_event.expect("ok"),K8Watch::MODIFIED(_)));

            let mut delete_events = service_streams.next().await.expect("events").expect("events");
            let delete_event= delete_events.pop().unwrap();
            assert!(matches!(delete_event.expect("ok"),K8Watch::DELETED(_)));

        }



    }

    #[test_async]
    async fn test_service_changes() -> Result<(), ClientError> {
        
        let client = create_client();
        
        join(generate_services_data(&client),verify_client(&client)).await;

        Ok(())
    }


}
