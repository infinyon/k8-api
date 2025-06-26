#[cfg(feature = "k8")]
mod canary_test {

    use anyhow::Result;
    use tracing::debug;
    use tracing::info;

    use fluvio_future::test_async;
    use k8_client::K8Client;
    use k8_metadata_client::MetadataClient;
    use k8_metadata_client::NameSpace;
    use k8_types::core::service::ServiceSpec;
    use k8_types::K8Obj;

    // get services to find kubernetes api
    #[test_async]
    async fn test_client_server_version() -> Result<()> {
        let client = K8Client::try_default().expect("cluster could not be configured");
        let version = client.server_version().await?;
        info!(
            "server version reported: {}.{}",
            version.major, version.minor,
        );
        Ok(())
    }

    // get services to find kubernetes api
    #[test_async]
    async fn test_client_get_services() -> Result<()> {
        let client = K8Client::try_default().expect("cluster could not be configured");
        let services = client.retrieve_items::<ServiceSpec, _>("default").await?;
        debug!("service: {} has been retrieved", services.items.len());

        let kubernetes_service = services
            .items
            .iter()
            .find(|i| i.metadata.name == "kubernetes");
        assert!(kubernetes_service.is_some());
        Ok(())
    }

    use k8_types::core::secret::SecretSpec;

    #[test_async]
    async fn test_client_secrets() -> Result<()> {
        let client = K8Client::try_default().expect("cluster could not be configured");
        let secrets = client
            .retrieve_items::<SecretSpec, _>(NameSpace::All)
            .await
            .expect("item retrieve");

        let system_secrets: Vec<K8Obj<SecretSpec>> = secrets
            .items
            .into_iter()
            .filter(|s| s.metadata.namespace == "kube-system")
            .collect();

        info!(
            "system secrets: {} has been retrieved",
            system_secrets.len()
        );

        assert!(system_secrets.len() > 20);

        Ok(())
    }

    #[test_async]
    async fn test_pods() -> Result<()> {
        use k8_types::core::pod::PodSpec;

        let client = K8Client::try_default().expect("cluster could not be configured");
        let pod_items = client
            .retrieve_items::<PodSpec, _>("default")
            .await
            .expect("pods should exist");

        for pod in pod_items.items {
            println!("pod: {pod:#?}");
        }

        Ok(())
    }
}
