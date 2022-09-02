use std::fs::read_to_string;
use std::fs::File;
use std::io::Result as IoResult;
use std::path::Path;

use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;

use crate::ConfigError;

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub cluster: ClusterDetail,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClusterDetail {
    pub insecure_skip_tls_verify: Option<bool>,
    pub certificate_authority: Option<String>,
    pub certificate_authority_data: Option<String>,
    pub server: String,
}

impl ClusterDetail {
    pub fn ca(&self) -> Option<IoResult<String>> {
        self.certificate_authority.as_ref().map(read_to_string)
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Context {
    pub name: String,
    pub context: ContextDetail,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct ContextDetail {
    pub cluster: String,
    pub user: String,
    pub namespace: Option<String>,
}

impl ContextDetail {
    pub fn namespace(&self) -> &str {
        match &self.namespace {
            Some(nm) => nm,
            None => "default",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct User {
    pub name: String,
    pub user: UserDetail,
}

//#[derive(Debug, PartialEq, Deserialize)]
//pub struct AuthProviderConfig {
//    pub name: String,
//    pub config: HashMap<String, String>,
//}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "name", content = "config")]
pub enum AuthProviderDetail {
    #[serde(alias = "gcp")]
    Gcp(GcpAuthProviderConfig),

    #[serde(other)]
    Other,
}

impl AuthProviderDetail {
    pub fn token(&self) -> Result<Option<String>, ConfigError> {
        if let AuthProviderDetail::Gcp(gcp_auth) = self {
            // Execute the command by default just in case access_key is expired
            let output = std::process::Command::new(&gcp_auth.cmd_path)
                .args(gcp_auth.cmd_args.split_whitespace().collect::<Vec<&str>>())
                .output()?;

            // Return token from json response
            if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                Ok(json["credential"]["access_token"]
                    .as_str()
                    .map(String::from))
            } else {
                Err(ConfigError::Other(
                    "Failed parsing request token response".to_string(),
                ))
            }
        } else {
            Err(ConfigError::Other(
                "Only Auth provider support for Google Kubernetes Engine (GKE). Please file issue."
                    .to_string(),
            ))
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GcpAuthProviderConfig {
    pub access_token: Option<String>,
    pub cmd_args: String,
    pub cmd_path: String,
    pub expiry: Option<String>,
    pub expiry_key: String,
    pub token_key: String,
}

//#[derive(Debug, PartialEq, Deserialize)]
//#[serde(rename_all = "kebab-case")]
//pub struct OidcAuthProviderConfig {
//    client_id: String,
//    client_secret: String,
//    id_token: String,
//    idp_certificate_authority: String,
//    idp_issuer_url: String,
//    refresh_token: String,
//}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UserDetail {
    pub auth_provider: Option<AuthProviderDetail>,
    pub client_certificate: Option<String>,
    pub client_key: Option<String>,
    pub client_certificate_data: Option<String>,
    pub client_key_data: Option<String>,
    pub exec: Option<Exec>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Exec {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub args: Vec<String>,
    pub command: String,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KubeConfig {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub clusters: Vec<Cluster>,
    pub contexts: Vec<Context>,
    pub current_context: String,
    pub kind: String,
    pub users: Vec<User>,
}

impl KubeConfig {
    /// read from default home directory
    pub fn from_home() -> Result<Self, ConfigError> {
        let home_dir = home_dir().unwrap();
        Self::from_file(home_dir.join(".kube").join("config"))
    }

    pub fn from_file<T: AsRef<Path>>(path: T) -> Result<Self, ConfigError> {
        let file = File::open(path)?;
        Ok(serde_yaml::from_reader(file)?)
    }

    pub fn current_context(&self) -> Option<&Context> {
        self.contexts
            .iter()
            .find(|c| c.name == self.current_context)
    }

    pub fn current_cluster(&self) -> Option<&Cluster> {
        if let Some(ctx) = self.current_context() {
            self.clusters.iter().find(|c| c.name == ctx.context.cluster)
        } else {
            None
        }
    }

    pub fn current_user(&self) -> Option<&User> {
        if let Some(ctx) = self.current_context() {
            self.users.iter().find(|c| c.name == ctx.context.user)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {

    use super::KubeConfig;

    #[test]
    fn test_decode_default_config() {
        let config = KubeConfig::from_file("data/k8config.yaml").expect("read");
        assert_eq!(config.api_version, "v1");
        assert_eq!(config.kind, "Config");
        assert_eq!(config.current_context, "flv");
        assert_eq!(config.clusters.len(), 1);
        let cluster = &config.clusters[0].cluster;
        assert_eq!(cluster.server, "https://192.168.0.0:8443");
        assert_eq!(
            cluster.certificate_authority,
            Some("/Users/test/.minikube/ca.crt".to_owned())
        );
        assert_eq!(config.contexts.len(), 2);
        let ctx = &config.contexts[0].context;
        assert_eq!(ctx.cluster, "minikube");
        assert_eq!(ctx.namespace.as_ref().unwrap(), "flv");

        let current_cluster = config.current_cluster().expect("current");
        assert_eq!(current_cluster.name, "minikube");
    }
}
