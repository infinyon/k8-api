#[cfg(feature = "k8")]
#[cfg(not(feature = "k8_stream"))]
mod integration_tests {

    use k8_client::K8Client;
    use k8_obj_app::stateful::StatefulSetSpec;
    use k8_client::ClientError;
    use flv_future_aio::test_async;
    use k8_client::fixture::TEST_NS;
    use k8_metadata_client::MetadataClient;

    // this assume we have at least one statefulset
    //#[test_async]
    async fn test_client_get_statefulset() -> Result<(), ClientError> {
        let client = K8Client::default().expect("cluster could not be configured");
        client.retrieve_items::<StatefulSetSpec,_>(TEST_NS).await?;
        Ok(()) as Result<(), ClientError>
    }
}
