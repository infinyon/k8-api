use k8_config::KubeConfig;

const KUBECONFIG: &str = "KUBECONFIG";

fn main() {
    // Read the KUBECONFIG env var for a path, or attempt to open $HOME/.kube/config
    // Parse then print config

    fluvio_future::subscriber::init_tracer(None);
    let config = std::env::var(KUBECONFIG)
        .map_or(KubeConfig::from_home(), KubeConfig::from_file)
        .expect("Load failed");

    println!("{config:#?}")
}
