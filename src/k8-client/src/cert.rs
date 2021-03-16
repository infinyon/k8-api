use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::Path;

use tracing::debug;
use tracing::trace;

use k8_config::K8Config;
use k8_config::KubeConfig;
use k8_config::PodConfig;

use crate::ClientError;

pub trait ConfigBuilder: Sized {
    type Client;

    fn new() -> Self;

    fn build(self) -> Result<Self::Client, ClientError>;

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> Result<Self, IoError>;

    // load client certificate (crt) and private key
    fn load_client_certificate<P: AsRef<Path>>(
        self,
        client_crt_path: P,
        client_key_path: P,
    ) -> Result<Self, IoError>;
}

/// Build Client
#[derive(Debug)]
pub struct ClientConfigBuilder<B> {
    config: K8Config,
    builder: B,
    external_token: Option<String>,
}

impl<B> ClientConfigBuilder<B>
where
    B: ConfigBuilder,
{
    pub fn new(config: K8Config) -> Result<Self, IoError> {
        let (builder, external_token) = Self::config(&config)?;

        Ok(Self {
            config,
            builder,
            external_token,
        })
    }

    /// configure based con k8 config
    fn config(config: &K8Config) -> Result<(B, Option<String>), IoError> {
        let builder = B::new();
        match config {
            K8Config::Pod(pod_config) => {
                Ok((Self::configure_in_cluster(builder, pod_config)?, None))
            }
            K8Config::KubeConfig(kube_config) => {
                Self::configure_out_of_cluster(builder, &kube_config.config)
            }
        }
    }

    pub fn k8_config(&self) -> &K8Config {
        &self.config
    }

    pub fn token(&self) -> Option<String> {
        if let Some(token) = &self.external_token {
            Some(token.clone())
        } else {
            match self.k8_config() {
                K8Config::Pod(pod) => Some(pod.token.to_owned()),
                _ => None,
            }
        }
    }

    pub fn host(&self) -> String {
        self.k8_config().api_path().to_owned()
    }

    pub fn build(self) -> Result<B::Client, ClientError> {
        self.builder.build()
    }

    fn configure_in_cluster(builder: B, pod: &PodConfig) -> Result<B, IoError> {
        debug!("configure as pod in cluster");
        let path = pod.ca_path();
        debug!("loading ca at: {}", path);
        builder.load_ca_certificate(path)
    }

    fn configure_out_of_cluster(
        builder: B,
        kube_config: &KubeConfig,
    ) -> Result<(B, Option<String>), IoError> {
        use std::io::Write;
        use std::process::Command;

        use base64::decode;
        use rand::distributions::Alphanumeric;
        use rand::Rng;

        use crate::k8_types::core::plugin::ExecCredentialSpec;
        use crate::k8_types::K8Obj;

        let current_user = kube_config.current_user().ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidInput,
                "config must have current user".to_owned(),
            )
        })?;

        let current_cluster = kube_config.current_cluster().ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidInput,
                "config must have current cluster".to_owned(),
            )
        })?;

        if let Some(ca_data) = &current_cluster.cluster.certificate_authority_data {
            let pem_bytes = decode(ca_data).unwrap();

            trace!("pem: {}", String::from_utf8_lossy(&pem_bytes).to_string());

            let random_file_name = format!(
                "eks-pem-{}",
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .map(char::from)
                    .take(15)
                    .collect::<String>()
                    .to_lowercase()
            );

            let file_path = std::env::temp_dir().join(random_file_name);
            debug!("writing pem file to: {:#?}", file_path);

            let mut file = std::fs::File::create(file_path.clone())?;
            file.write_all(&pem_bytes)?;

            let builder = builder.load_ca_certificate(file_path)?;

            if let Some(exec) = &current_user.user.exec {
                let token_output = Command::new(exec.command.clone())
                    .args(exec.args.clone())
                    .output()?;

                debug!(
                    "token: {}",
                    String::from_utf8_lossy(&token_output.stdout).to_string()
                );
                let credential: K8Obj<ExecCredentialSpec> =
                    serde_json::from_slice(&token_output.stdout)?;
                let token = credential.status.token;
                debug!("token: {:#?}", token);
                Ok((builder, Some(token)))
            } else {
                Ok((builder, None))
            }
        } else {
            let builder =
                if let Some(client_crt_path) = current_user.user.client_certificate.as_ref() {
                    if let Some(client_key_path) = current_user.user.client_key.as_ref() {
                        debug!(
                            "loading client crt: {} and client key: {}",
                            client_crt_path, client_key_path
                        );
                        builder.load_client_certificate(client_crt_path, client_key_path)?
                    } else {
                        return Err(IoError::new(
                            ErrorKind::InvalidInput,
                            "no client cert key path founded".to_owned(),
                        ));
                    }
                } else {
                    return Err(IoError::new(
                        ErrorKind::InvalidInput,
                        "no client cert crt path founded".to_owned(),
                    ));
                };

            let ca_certificate_path = current_cluster
                .cluster
                .certificate_authority
                .as_ref()
                .ok_or_else(|| {
                    IoError::new(
                        ErrorKind::InvalidInput,
                        "current cluster must have CA crt path".to_owned(),
                    )
                })?;

            Ok((builder.load_ca_certificate(ca_certificate_path)?, None))
        }
    }
}
