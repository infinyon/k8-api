# Kubernetes Rust Client


This is similar to Kubernetes Go Client:  https://github.com/kubernetes/client-go

Example of using the client:

```
use k8_client::K8Client;
use k8_client::pod::{PodSpec,PodStatus};

async fn main() {

    let client = K8Client::default().expect("cluster not initialized");

    let pods = client.retrieve_item<PodSpec>().await.expect("pods should exist");

    for pod in pods {
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
