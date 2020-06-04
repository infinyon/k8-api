# Kubernetes Rust Client


This is similar to Kubernetes Go Client:  https://github.com/kubernetes/client-go

Example of using the client:

```
use k8_client::K8Client;
use k8_obj_core::pod::{PodSpec,PodStatus};

async fn main() {

    let client = K8Client::default().expect("cluster not initialized");

    let pod_items = client.retrieve_items::<PodSpec,_>("default").await.expect("pods should exist");

    for pod in pod_items.items {
        println!("pod: {:#?}",pod);
    }
}

```


## License

This project is licensed under the [Apache license](LICENSE-APACHE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Fluvio by you, shall be licensed as Apache, without any additional
terms or conditions.
