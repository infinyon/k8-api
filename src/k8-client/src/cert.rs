use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::Path;

use tracing::{debug, error};

use k8_config::K8Config;
use k8_config::KubeConfig;
use k8_config::PodConfig;
use k8_config::{GcpAuthProviderConfig, AuthProviderDetail};
use serde_json::Value;

use crate::ClientError;

pub trait ConfigBuilder: Sized {
    type Client;

    fn new() -> Self;

    fn build(self) -> Result<Self::Client, ClientError>;

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> Result<Self, IoError>;

    // load from ca data
    fn load_ca_cert_with_data(self, data: Vec<u8>) -> Result<Self, IoError>;

    // load client certificate (crt) and private key
    fn load_client_certificate<P: AsRef<Path>>(
        self,
        client_crt_path: P,
        client_key_path: P,
    ) -> Result<Self, IoError>;

    fn load_client_certificate_with_data(
        self,
        client_crt: Vec<u8>,
        client_key: Vec<u8>,
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
        } else if let K8Config::KubeConfig(context) = &self.k8_config() {
            // Look for auth-provider for dynamic token
            // We should be able to know from the User config if we need to call gcp for a token

            let kube_config = &context.config;

            //    // Get name of current context
            let current_context = kube_config.current_context.clone();

            if let Some(c) = &kube_config
                .contexts
                .iter()
                .find(|context| &context.name == &current_context)
            {
                let users = &kube_config.users;

                let token = if let Some(u) = users.iter().find(|user| &user.name == &c.context.user)
                {
                    //Some(&u.user)

                    if let Some(auth_provider) = &u.user.auth_provider {
                        // If GCP
                        if let AuthProviderDetail::Gcp(gcp_auth) = auth_provider {
                            debug!("{gcp_auth:#?}");

                            // Execute the command by default just in case access_key is expired
                            let output = std::process::Command::new(&gcp_auth.cmd_path)
                                .args(gcp_auth.cmd_args.split_whitespace().collect::<Vec<&str>>())
                                .output()
                                .expect("gcp command failed");

                            // Return token from json response
                            if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                                debug!("{json:#?}");
                                debug!("{:#?}", json["credential"]["access_token"]);

                                json["credential"]["access_token"]
                                    .as_str()
                                    .map(String::from)
                            } else {
                                None
                            }
                        } else {
                            error!("Only Auth provider support for GCP");
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                token

                // This is the `access_token` value from the gcloud cli command, which is not the same value as what is in the User auth-provider
                //Some("ya29.A0ARrdaM8BR5Oi_TctugLqKhyud3Cqauv_Aj2e0FcOCnXDMyV26f7xE4f7GqMQGCO6yjTO2Iv3rt7yRuNgzOVmWDPXgLRfbTBrqAi82j_FlVYhS7sjHWEJfurt2B3hOOZkbZEBMwZ7lsMV2vrq1XBzwJytUGxSBviUN2h3pg".to_string())

                //Some((c, user)
            } else {
                None
            }
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
        use std::process::Command;

        use base64::decode;

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

        // load CA cluster

        let builder = if let Some(ca_data) = &current_cluster.cluster.certificate_authority_data {
            debug!("detected in-line cluster CA certs");
            let pem_bytes = decode(ca_data).unwrap();
            builder.load_ca_cert_with_data(pem_bytes)?
        } else {
            // let not inline, then must must ref to file
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

            debug!("loading cluster CA from: {:#?}", ca_certificate_path);

            builder.load_ca_certificate(ca_certificate_path)?
        };

        // Note: GCP kubernetes clusters don't have any of these set on user
        // load client certs
        if let Some(exec) = &current_user.user.exec {
            debug!(exec = ?exec,"loading client CA using exec");

            let token_output = Command::new(exec.command.clone())
                .args(exec.args.clone())
                .output()?;

            debug!(
                cmd_token = ?String::from_utf8_lossy(&token_output.stdout).to_string()
            );

            let credential: K8Obj<ExecCredentialSpec> =
                serde_json::from_slice(&token_output.stdout)?;
            let token = credential.status.token;
            debug!(?token);
            Ok((builder, Some(token)))
        } else if let Some(client_cert_data) = &current_user.user.client_certificate_data {
            debug!("detected in-line cluster CA certs");
            let client_cert_pem_bytes = decode(client_cert_data).map_err(|err| {
                IoError::new(
                    ErrorKind::InvalidInput,
                    format!("base64 decoding err: {}", err),
                )
            })?;

            let client_key_pem_bytes =
                decode(current_user.user.client_key_data.as_ref().ok_or_else(|| {
                    IoError::new(
                        ErrorKind::InvalidInput,
                        "current user must have client key data".to_owned(),
                    )
                })?)
                .map_err(|err| {
                    IoError::new(
                        ErrorKind::InvalidInput,
                        format!("base64 decoding err: {}", err),
                    )
                })?;

            Ok((
                builder.load_client_certificate_with_data(
                    client_cert_pem_bytes,
                    client_key_pem_bytes,
                )?,
                None,
            ))
        } else if let Some(client_crt_path) = current_user.user.client_certificate.as_ref() {
            let client_key_path = current_user.user.client_key.as_ref().ok_or_else(|| {
                IoError::new(
                    ErrorKind::InvalidInput,
                    "current user must have client key".to_owned(),
                )
            })?;

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
        } else {
            // TODO: Uncomment this out when we support alternate auth provider flow
            //Err(IoError::new(
            //    ErrorKind::InvalidInput,
            //    "no client cert crt data, path or user token were found".to_owned(),
            //))

            Ok((builder, None))
        }
    }
}
