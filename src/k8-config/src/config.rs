use std::collections::HashMap;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Result as IoResult;
use std::path::Path;

use dirs::home_dir;
use serde::Deserialize;

use crate::ConfigError;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub cluster: ClusterDetail,
}

#[derive(Debug, PartialEq, Deserialize)]
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

#[derive(Debug, PartialEq, Deserialize)]
pub struct Context {
    pub name: String,
    pub context: ContextDetail,
}

#[derive(Debug, PartialEq, Deserialize)]
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

#[derive(Debug, PartialEq, Deserialize)]
pub struct User {
    pub name: String,
    pub user: UserDetail,
}

// Maybe we should take a step back and support a simple
#[derive(Debug, PartialEq, Deserialize)]
pub struct AuthProviderConfig {
    pub name: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "name", content = "config")]
pub enum AuthProviderDetail {
    #[serde(alias = "gcp")]
    Gcp(GcpAuthProviderConfig),

    #[serde(other)]
    Other, // Not prepared to support oidc

           // When ready, serde_enum_str might be helpful for handling this case
           //Other(AuthProviderConfig), // Not prepared to support oidc
           //#[serde(alias = "oidc")]
           //Oidc(OidcAuthProviderConfig)
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GcpAuthProviderConfig {
    pub access_token: String,
    pub cmd_args: String,
    pub cmd_path: String,
    pub expiry: String,
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

#[derive(Debug, PartialEq, Deserialize)]
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

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Exec {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub args: Vec<String>,
    pub command: String,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KubeConfig {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub clusters: Vec<Cluster>, // I might consider storing this in a HashMap
    pub contexts: Vec<Context>, // This too
    pub current_context: String,
    pub kind: String,
    pub users: Vec<User>, // This too
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
