use k8_config::context::MinikubeContext;
use k8_config::ConfigError;

/// Performs following
///     add minikube IP address to /etc/host
///     create new kubectl cluster and context which uses minikube name
fn main() {
    if let Err(e) = run() {
        println!("{}", e);
    }
}

fn run() -> Result<(), ConfigError> {
    let context = MinikubeContext::try_from_system()?;
    context.save()?;
    Ok(())
}
