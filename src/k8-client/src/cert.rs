use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use tracing::debug;

use k8_config::K8Config;
use k8_config::KubeConfig;
use k8_config::PodConfig;
use k8_config::AuthProviderDetail;

pub trait ConfigBuilder: Sized {
    type Client;

    fn new() -> Self;

    fn build(self) -> Result<Self::Client>;

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> Result<Self>;

    // load from ca data
    fn load_ca_cert_with_data(self, data: Vec<u8>) -> Result<Self>;

    // load client certificate (crt) and private key
    fn load_client_certificate<P: AsRef<Path>>(
        self,
        client_crt_path: P,
        client_key_path: P,
    ) -> Result<Self>;

    fn load_client_certificate_with_data(
        self,
        client_crt: Vec<u8>,
        client_key: Vec<u8>,
    ) -> Result<Self>;
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
    pub fn new(config: K8Config) -> Result<Self> {
        let (builder, external_token) = Self::config(&config)?;

        Ok(Self {
            config,
            builder,
            external_token,
        })
    }

    /// configure based con k8 config
    fn config(config: &K8Config) -> Result<(B, Option<String>)> {
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

    pub fn token(&self) -> Result<Option<String>> {
        if let Some(token) = &self.external_token {
            Ok(Some(token.clone()))
        } else if let K8Config::KubeConfig(context) = &self.k8_config() {
            // We should be able to know if we use dynamic tokens from the User config if using `auth_provider`

            let kube_config = &context.config;

            let current_context = kube_config.current_context.clone();

            if let Some(c) = &kube_config
                .contexts
                .iter()
                .find(|context| context.name == current_context)
            {
                let users = &kube_config.users;

                let token = if let Some(u) = users.iter().find(|user| user.name == c.context.user) {
                    if let Some(auth_provider) = &u.user.auth_provider {
                        auth_provider.token()?
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(token)
            } else {
                Ok(None)
            }
        } else {
            match self.k8_config() {
                K8Config::Pod(pod) => Ok(Some(pod.token.to_owned())),
                _ => Ok(None),
            }
        }
    }

    pub fn host(&self) -> String {
        self.k8_config().api_path().to_owned()
    }

    pub fn build(self) -> Result<B::Client> {
        self.builder.build()
    }

    fn configure_in_cluster(builder: B, pod: &PodConfig) -> Result<B> {
        debug!("configure as pod in cluster");
        let path = pod.ca_path();
        debug!("loading ca at: {}", path);
        builder.load_ca_certificate(path)
    }

    fn configure_out_of_cluster(
        builder: B,
        kube_config: &KubeConfig,
    ) -> Result<(B, Option<String>)> {
        use std::process::Command;

        use base64::prelude::{Engine, BASE64_STANDARD};

        use k8_types::core::plugin::ExecCredentialSpec;
        use k8_types::K8Obj;

        let current_user = kube_config
            .current_user()
            .ok_or_else(|| anyhow!("config must have current user"))?;

        let current_cluster = kube_config
            .current_cluster()
            .ok_or_else(|| anyhow!("config must have current cluster"))?;

        // load CA cluster

        let builder = if let Some(ca_data) = &current_cluster.cluster.certificate_authority_data {
            debug!("detected in-line cluster CA certs");
            let pem_bytes = BASE64_STANDARD.decode(ca_data).unwrap();
            builder.load_ca_cert_with_data(pem_bytes)?
        } else {
            // let not inline, then must must ref to file
            if let Some(ca_certificate_path) =
                current_cluster.cluster.certificate_authority.as_ref()
            {
                debug!("loading cluster CA from: {:#?}", ca_certificate_path);
                builder.load_ca_certificate(ca_certificate_path)?
            } else {
                return Ok((builder, None));
            }
        };

        // load client certs
        // Note: Google Kubernetes (GKE) clusters don't have any of these set on user
        if let Some(exec) = &current_user.user.exec {
            debug!(exec = ?exec,"loading client CA using exec");

            let token_output = Command::new(exec.command.clone())
                .args(exec.args.clone())
                .output()?;

            debug!(
                cmd_token = ?String::from_utf8_lossy(&token_output.stdout).to_string()
            );

            let credential: K8Obj<ExecCredentialSpec> =
                serde_json::from_slice(&token_output.stdout).map_err(|err| {
                    let cmd_token = String::from_utf8_lossy(&token_output.stdout).to_string();
                    anyhow!(
                        "error parsing credential from: {} {}\nreply: {}\nerr: {}",
                        exec.command,
                        exec.args.join(" "),
                        cmd_token,
                        err
                    )
                })?;
            let token = credential.status.token;
            debug!(?token);
            Ok((builder, Some(token)))
        } else if let Some(client_cert_data) = &current_user.user.client_certificate_data {
            debug!("detected in-line cluster CA certs");
            let client_cert_pem_bytes = BASE64_STANDARD
                .decode(client_cert_data)
                .context("base64 decoding err")?;

            let client_key_pem_bytes = BASE64_STANDARD
                .decode(
                    current_user
                        .user
                        .client_key_data
                        .as_ref()
                        .ok_or_else(|| anyhow!("current user must have client key data"))?,
                )
                .context("base64 decoding err")?;

            Ok((
                builder.load_client_certificate_with_data(
                    client_cert_pem_bytes,
                    client_key_pem_bytes,
                )?,
                None,
            ))
        } else if let Some(client_crt_path) = current_user.user.client_certificate.as_ref() {
            let client_key_path = current_user
                .user
                .client_key
                .as_ref()
                .ok_or_else(|| anyhow!("current user must have client key"))?;

            debug!(
                "loading client crt: {} and client key: {}",
                client_crt_path, client_key_path
            );
            Ok((
                builder.load_client_certificate(client_crt_path, client_key_path)?,
                None,
            ))
        } else if let Some(user_token) = &current_user.user.token {
            Ok((builder, Some(user_token.clone())))
        } else if let Some(AuthProviderDetail::Gcp(_)) = &current_user.user.auth_provider {
            Ok((builder, None))
        } else {
            Err(anyhow!(
                "no client cert crt data, path or user token were found"
            ))
        }
    }
}
