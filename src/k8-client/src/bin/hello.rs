use tracing::debug;

use fluvio_future::task::run_block_on;
use k8_client::ClientError;
use k8_client::K8Client;
use k8_metadata_client::MetadataClient;
use k8_obj_core::service::ServiceSpec;

async fn test_client_get_services() -> Result<(), ClientError> {
    let client = K8Client::default().expect("cluster could not be configured");
    let services = client.retrieve_items::<ServiceSpec, _>("default").await.expect("items retrieved");
    debug!("service: {} has been retrieved", services.items.len());

    let kubernetes_service = services
        .items
        .iter()
        .find(|i| i.metadata.name == "kubernetes");
    assert!(kubernetes_service.is_some());
    Ok(())
}

/// run in the K8 as pod
/// same as canary
/// 
fn main() {
    fluvio_future::subscriber::init_tracer(None);
    run_block_on(test_client_get_services()).expect("success");
}
