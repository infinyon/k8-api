#[cfg(feature = "k8")]
mod integration_tests {

    use std::collections::HashMap;
    use std::time::Duration;

    use tracing::debug;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use once_cell::sync::Lazy;

    use fluvio_future::test_async;
    use fluvio_future::timer::sleep;

    use k8_client::{ K8Client, ClientError} ;
    use k8_metadata_client::{ MetadataClient, ApplyResult};
    use k8_obj_core::service::ServicePort;
    use k8_obj_core::service::ServiceSpec;
    use k8_obj_metadata::{ InputK8Obj, InputObjectMeta, Spec};

    const SPU_DEFAULT_NAME: &'static str = "spu";
    const PORT: u16 = 9002;
  
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

        // go thru create, update spec and delete
        for i in 0..10 {
        
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

            debug!("updated item: {:#?}",update_item);
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

    }

    #[test_async]
    async fn test_service_changes() -> Result<(), ClientError> {
        
        let client = create_client();
        
        generate_services_data(&client).await;

        Ok(())
    }


}
