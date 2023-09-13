use std::fs::read_to_string;
use std::fs::File;
use std::io::Result as IoResult;
use std::path::Path;
use std::path::PathBuf;

use dirs::home_dir;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::ConfigError;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub cluster: ClusterDetail,
}

#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClusterDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure_skip_tls_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_authority_data: Option<String>,
    pub server: String,
}

impl ClusterDetail {
    pub fn ca(&self) -> Option<IoResult<String>> {
        self.certificate_authority.as_ref().map(read_to_string)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub name: String,
    pub context: ContextDetail,
}

#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct ContextDetail {
    pub cluster: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub user: UserDetail,
}

//#[derive(Debug, PartialEq, Deserialize)]
//pub struct AuthProviderConfig {
//    pub name: String,
//    pub config: HashMap<String, String>,
//}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GcpAuthProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub cmd_args: String,
    pub cmd_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UserDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<AuthProviderDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<Exec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Exec {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub args: Vec<String>,
    pub command: String,
}

#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KubeConfig {
    #[serde(skip)]
    pub path: PathBuf,
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
        let file = File::open(path.as_ref())?;
        let mut config: Self = serde_yaml::from_reader(file)?;
        config.path = path.as_ref().to_path_buf();
        Ok(config)
    }

    pub fn to_file<T: AsRef<Path>>(&self, path: T) -> Result<(), ConfigError> {
        let file = File::create(path)?;
        Ok(serde_yaml::to_writer(file, self)?)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        self.to_file(&self.path)
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

    pub fn put_user(&mut self, user: User) -> Option<User> {
        let prev = self.users.iter_mut().find(|u| u.name.eq(&user.name));
        match prev {
            Some(prev) => Some(std::mem::replace(prev, user)),
            None => {
                self.users.push(user);
                None
            }
        }
    }

    pub fn put_cluster(&mut self, cluster: Cluster) -> Option<Cluster> {
        let prev = self.clusters.iter_mut().find(|c| c.name.eq(&cluster.name));
        match prev {
            Some(prev) => Some(std::mem::replace(prev, cluster)),
            None => {
                self.clusters.push(cluster);
                None
            }
        }
    }

    pub fn put_context(&mut self, context: Context) -> Option<Context> {
        let prev = self.contexts.iter_mut().find(|c| c.name.eq(&context.name));
        match prev {
            Some(prev) => Some(std::mem::replace(prev, context)),
            None => {
                self.contexts.push(context);
                None
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{User, Cluster, Context};

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

    #[test]
    fn test_config_ser() {
        //given
        let config = KubeConfig::from_file("data/k8config.yaml").expect("read");

        //when
        let serialized = serde_yaml::to_string(&config).expect("serialized");

        //then
        assert_eq!(
            serialized,
            r#"apiVersion: v1
clusters:
- name: minikube
  cluster:
    certificate-authority: /Users/test/.minikube/ca.crt
    server: https://192.168.0.0:8443
contexts:
- name: flv
  context:
    cluster: minikube
    user: minikube
    namespace: flv
- name: minikube
  context:
    cluster: minikube
    user: minikube
current-context: flv
kind: Config
users:
- name: minikube
  user:
    client-certificate: /Users/test/.minikube/client.crt
    client-key: /Users/test/.minikube/client.key
"#
        );
    }

    #[test]
    fn test_put_user() {
        //given
        let mut config = KubeConfig::default();

        let user1 = User {
            name: "user1".to_string(),
            user: crate::UserDetail {
                username: Some("username1".to_string()),
                ..Default::default()
            },
        };

        let user1_2 = User {
            name: "user1".to_string(),
            user: crate::UserDetail {
                username: Some("username2".to_string()),
                ..Default::default()
            },
        };

        let user2 = User {
            name: "user2".to_string(),
            user: Default::default(),
        };

        //when
        assert!(config.put_user(user1).is_none());
        assert!(config.put_user(user2).is_none());

        let prev = config.put_user(user1_2);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().user.username.unwrap(), "username1");
    }

    #[test]
    fn test_put_cluster() {
        //given
        let mut config = KubeConfig::default();

        let cluster1 = Cluster {
            name: "cluster1".to_string(),
            cluster: crate::ClusterDetail {
                server: "server1".to_string(),
                ..Default::default()
            },
        };

        let cluster1_2 = Cluster {
            name: "cluster1".to_string(),
            cluster: crate::ClusterDetail {
                server: "server2".to_string(),
                ..Default::default()
            },
        };

        let cluster2 = Cluster {
            name: "cluster2".to_string(),
            cluster: Default::default(),
        };

        //when
        assert!(config.put_cluster(cluster1).is_none());
        assert!(config.put_cluster(cluster2).is_none());

        let prev = config.put_cluster(cluster1_2);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().cluster.server, "server1");
    }

    #[test]
    fn test_put_context() {
        //given
        let mut config = KubeConfig::default();

        let context1 = Context {
            name: "context1".to_string(),
            context: crate::ContextDetail {
                cluster: "cluster1".to_string(),
                ..Default::default()
            },
        };

        let context1_2 = Context {
            name: "context1".to_string(),
            context: crate::ContextDetail {
                cluster: "cluster2".to_string(),
                ..Default::default()
            },
        };

        let context2 = Context {
            name: "context2".to_string(),
            context: Default::default(),
        };

        //when
        assert!(config.put_context(context1).is_none());
        assert!(config.put_context(context2).is_none());

        let prev = config.put_context(context1_2);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().context.cluster, "cluster1");
    }
}
