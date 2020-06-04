

/// Performs following
///     add minikube IP address to /etc/host
///     create new kubectl cluster and context which uses minikube name
fn main()  {

    use k8_config::context::create_dns_context;
    use k8_config::context::Option;

    create_dns_context(Option::default())
    
}
