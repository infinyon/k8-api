use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::process::{Command, Stdio};

use crate::ConfigError;
use serde::Deserialize;
use tracing::debug;

pub use v1::*;

/// Configuration options to wire up Fluvio on Minikube
///
/// # Example
///
/// To read the current minikube configuration and save it
/// to the kubernetes `kubectl` context, do the following:
///
/// ```no_run
/// use k8_config::context::MinikubeContext;
///
/// // Load the context from the system
/// let context = MinikubeContext::try_from_system().unwrap();
///
/// // Save the context configuration
/// context.save().unwrap();
/// ```
pub struct MinikubeContext {
    name: String,
    profile: MinikubeProfile,
}

impl MinikubeContext {
    /// Attempts to derive a `MinikubeContext` from the system
    ///
    /// This requires the presence of the `minikube` executable,
    /// which will tell us the current IP and port that minikube
    /// is running on.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use k8_config::context::MinikubeContext;
    /// let context = MinikubeContext::try_from_system().unwrap();
    /// ```
    pub fn try_from_system() -> Result<Self, ConfigError> {
        Ok(Self {
            name: "flvkube".to_string(),
            profile: MinikubeProfile::load()?,
        })
    }

    /// Sets the name of the context
    ///
    /// # Example
    ///
    /// ```no_run
    /// use k8_config::context::MinikubeContext;
    /// let context = MinikubeContext::try_from_system().unwrap()
    ///     .with_name("my-minikube");
    /// ```
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = name.into();
        self
    }

    /// Saves the Minikube context for kubectl and updates the minikube IP
    ///
    /// # Example
    ///
    /// ```no_run
    /// use k8_config::context::MinikubeContext;
    /// let context = MinikubeContext::try_from_system().unwrap();
    /// context.save().unwrap();
    /// ```
    pub fn save(&self) -> Result<(), ConfigError> {
        // Check if the detected minikube IP matches the /etc/hosts entry
        if !self.profile.matches_hostfile()? {
            // If the /etc/hosts file is not up to date, update it
            debug!("hosts file is outdated: updating");
            self.update_hosts()?;
        }
        self.update_kubectl_context()?;
        Ok(())
    }

    /// Updates the `kubectl` context to use the current settings
    fn update_kubectl_context(&self) -> Result<(), ConfigError> {
        Command::new("kubectl")
            .args(&["config", "set-cluster", &self.name])
            .arg(&format!(
                "--server=https://minikubeCA:{}",
                self.profile.port()
            ))
            .arg(&format!("--certificate-authority={}", load_cert_auth()))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Command::new("kubectl")
            .args(&["config", "set-context", &self.name])
            .arg("--user=minikube")
            .arg(&format!("--cluster={}", &self.name))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Command::new("kubectl")
            .args(&["config", "use-context", &self.name])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(())
    }

    /// Updates the `/etc/hosts` file by rewriting the line with `minikubeCA`
    fn update_hosts(&self) -> Result<(), ConfigError> {
        let render = format!(
            r#"
#!/bin/bash
# Get IP from context, if available
export IP={ip}
# If there is no IP in context, use "minikube ip"
export IP="${{IP:-$(minikube ip)}}"
sudo sed -i'' -e '/minikubeCA/d' /etc/hosts
echo "$IP minikubeCA" | sudo tee -a  /etc/hosts
"#,
            ip = &self.profile.ip()
        );

        let tmp_file = env::temp_dir().join("flv_minikube.sh");

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o755)
            .open(tmp_file.clone())
            .expect("temp script can't be created");

        file.write_all(render.as_bytes())
            .expect("file write failed");

        file.sync_all().expect("sync");
        drop(file);

        debug!("script {}", render);

        Command::new(tmp_file)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MinikubeNode {
    #[serde(rename = "IP")]
    ip: IpAddr,
    #[serde(rename = "Port")]
    port: u16,
}

#[derive(Debug, Deserialize)]
struct MinikubeConfig {
    #[serde(rename = "Name")]
    _name: String,
    #[serde(rename = "Nodes")]
    nodes: Vec<MinikubeNode>,
}

#[derive(Debug, Deserialize)]
struct MinikubeProfileWrapper {
    valid: Vec<MinikubeProfileJson>,
}

#[derive(Debug, Deserialize)]
struct MinikubeProfileJson {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Status")]
    _status: String,
    #[serde(rename = "Config")]
    config: MinikubeConfig,
}

/// A description of the active Minikube instance, including IP and port
#[derive(Debug)]
struct MinikubeProfile {
    /// The name of the minikube profile, usually "minikube"
    _name: String,
    /// The active minikube node, with IP and port
    node: MinikubeNode,
}

impl MinikubeProfile {
    /// Gets minikube's current profile
    fn load() -> Result<MinikubeProfile, ConfigError> {
        let output = Command::new("minikube")
            .args(&["profile", "list", "-o", "json"])
            .output()?;
        let output_string = String::from_utf8(output.stdout).map_err(|e| {
            ConfigError::Other(format!(
                "`minikube profile list -o json` did not give UTF-8: {}",
                e
            ))
        })?;
        let profiles: MinikubeProfileWrapper =
            serde_json::from_str(&output_string).map_err(|e| {
                ConfigError::Other(format!(
                    "`minikube profile list -o json` did not give valid JSON: {}",
                    e
                ))
            })?;
        let profile_json = profiles
            .valid
            .into_iter()
            .next()
            .ok_or_else(|| ConfigError::Other("no valid minikube profiles".to_string()))?;
        let node = profile_json
            .config
            .nodes
            .into_iter()
            .next()
            .ok_or_else(|| ConfigError::Other("Minikube has no active nodes".to_string()))?;
        let profile = MinikubeProfile {
            _name: profile_json.name,
            node,
        };
        Ok(profile)
    }

    fn ip(&self) -> IpAddr {
        self.node.ip
    }

    fn port(&self) -> u16 {
        self.node.port
    }

    /// Checks whether the `/etc/hosts` file has an up-to-date entry for minikube
    ///
    /// Returns `Ok(true)` when the hostfile is up-to-date and no action is required.
    ///
    /// Returns `Ok(false)` when the hostfile is out of date or has no `minikubeCA` entry.
    /// In this case, the `/etc/hosts` file needs to be edited.
    ///
    /// Returns `Err(_)` when there is an error detecting the current Minikube ip address
    /// or if there is an error reading the hosts file.
    fn matches_hostfile(&self) -> Result<bool, ConfigError> {
        // Check if the /etc/hosts file matches the active node IP
        let matches = get_host_entry("minikubeCA")?
            .map(|ip| ip == self.node.ip)
            .unwrap_or(false);
        Ok(matches)
    }
}

/// Gets the current entry for a given host in `/etc/hosts` if there is one
fn get_host_entry(hostname: &str) -> Result<std::option::Option<IpAddr>, ConfigError> {
    // Get all of the host entries
    let hosts = hostfile::parse_hostfile()
        .map_err(|e| ConfigError::Other(format!("failed to get /etc/hosts entries: {}", e)))?;
    // Try to find a host entry with the given hostname
    let minikube_entry = hosts
        .into_iter()
        .find(|entry| entry.names.iter().any(|name| name == hostname));
    Ok(minikube_entry.map(|entry| entry.ip))
}

/// *Deprecated*: use [`MinikubeContext`] instead
///
/// Updates kubectl context settings
///
/// [`MinikubeContext`]: ./struct.MinikubeContext
pub mod v1 {
    use std::env;
    use std::fs::OpenOptions;
    use std::io;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::process::Command;

    use tera::Context;
    use tera::Tera;
    use tracing::debug;

    pub use crate::K8Config;

    pub(crate) fn load_cert_auth() -> String {
        let k8_config = K8Config::load().expect("loading");

        let ctx = match k8_config {
            K8Config::Pod(_) => panic!("should not be pod"),
            K8Config::KubeConfig(ctx) => ctx,
        };

        let config = ctx.config;

        let cluster = config
            .current_cluster()
            .expect("should have current context");

        cluster
            .cluster
            .certificate_authority
            .as_ref()
            .expect("certificate authority")
            .to_string()
    }

    #[deprecated(note = "Please use MinikubeContext instead")]
    pub struct Option {
        pub ctx_name: String,
    }

    #[allow(deprecated)]
    impl Default for Option {
        fn default() -> Self {
            Self {
                ctx_name: "flvkube".to_owned(),
            }
        }
    }

    /// create kube context that copy current cluster configuration
    #[allow(deprecated)]
    #[deprecated(note = "Please use MinikubeContext instead")]
    pub fn create_dns_context(option: Option) {
        const TEMPLATE: &str = r#"
#!/bin/bash
export IP=$(minikube ip)
sudo sed -i '' '/minikubeCA/d' /etc/hosts
echo "$IP minikubeCA" | sudo tee -a  /etc/hosts
cd ~
kubectl config set-cluster {{ name }} --server=https://minikubeCA:8443 --certificate-authority={{ ca }}
kubectl config set-context {{ name }} --user=minikube --cluster={{ name }}
kubectl config use-context {{ name }}
"#;

        let mut tera = Tera::default();

        tera.add_raw_template("cube.sh", TEMPLATE)
            .expect("string compilation");

        let mut context = Context::new();
        context.insert("name", &option.ctx_name);
        context.insert("ca", &load_cert_auth());

        let render = tera.render("cube.sh", &context).expect("rendering");

        let tmp_file = env::temp_dir().join("flv_minikube.sh");

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o755)
            .open(tmp_file.clone())
            .expect("temp script can't be created");

        file.write_all(render.as_bytes())
            .expect("file write failed");

        file.sync_all().expect("sync");
        drop(file);

        debug!("script {}", render);

        let output = Command::new(tmp_file).output().expect("cluster command");
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
    }
}
