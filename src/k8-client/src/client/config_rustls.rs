use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use futures_util::future::Future;
use futures_util::io::{AsyncRead as StdAsyncRead, AsyncWrite as StdAsyncWrite};
use http::Uri;
use tracing::debug;

use hyper::client::connect::{Connected, Connection};
use hyper::service::Service;
use hyper::Body;
use hyper::Client;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use rustls::WantsVerifier;
use rustls::client::WantsTransparencyPolicyOrClientCert;

use fluvio_future::net::TcpStream;
use fluvio_future::rust_tls::{
    ConnectorBuilder, ConnectorBuilderStage, DefaultClientTlsStream, TlsConnector,
    ConnectorBuilderWithConfig,
};
use super::executor::FluvioHyperExecutor;
use crate::cert::{ClientConfigBuilder, ConfigBuilder};

pub type HyperClient = Client<TlsHyperConnector, Body>;

pub type HyperConfigBuilder = ClientConfigBuilder<HyperClientBuilder>;

pub struct HyperTlsStream(DefaultClientTlsStream);

impl Connection for HyperTlsStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

impl AsyncRead for HyperTlsStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        match Pin::new(&mut self.0).poll_read(cx, buf.initialize_unfilled())? {
            Poll::Ready(bytes_read) => {
                buf.advance(bytes_read);
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for HyperTlsStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.0).poll_close(cx)
    }
}

/// hyper connector that uses fluvio TLS
#[derive(Clone)]
pub struct TlsHyperConnector(Arc<TlsConnector>);

impl TlsHyperConnector {
    fn new(connector: TlsConnector) -> Self {
        Self(Arc::new(connector))
    }
}

impl Service<Uri> for TlsHyperConnector {
    type Response = HyperTlsStream;
    type Error = anyhow::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let connector = self.0.clone();

        Box::pin(async move {
            let host = match uri.host() {
                Some(h) => h,
                None => return Err(anyhow!("no host")),
            };

            match uri.scheme_str() {
                Some("http") => Err(anyhow!("http not supported")),
                Some("https") => {
                    let socket_addr = {
                        let host = host.to_string();
                        let port = uri.port_u16().unwrap_or(443);
                        match (host.as_str(), port).to_socket_addrs()?.next() {
                            Some(addr) => addr,
                            None => return Err(anyhow!("host resolution: {} failed", host)),
                        }
                    };
                    debug!("socket address to: {}", socket_addr);
                    let tcp_stream = TcpStream::connect(&socket_addr).await?;

                    let stream = connector
                        .connect(host.try_into()?, tcp_stream)
                        .await
                        .map_err(|err| {
                            IoError::new(ErrorKind::Other, format!("tls handshake: {}", err))
                        })?;
                    Ok(HyperTlsStream(stream))
                }
                scheme => Err(anyhow!("{:?}", scheme)),
            }
        })
    }
}

pub enum ConnectorBuilderStages {
    WantsVerifier(ConnectorBuilderStage<WantsVerifier>),
    WantsTransparencyPolicyOrClientCert(ConnectorBuilderStage<WantsTransparencyPolicyOrClientCert>),
    ConnectorBuilder(ConnectorBuilderWithConfig),
}

impl ConnectorBuilderStages {
    pub fn build(self) -> Result<TlsConnector> {
        match self {
            Self::WantsVerifier(_) => {
                Err(IoError::new(ErrorKind::Other, "missing verifier").into())
            }
            Self::WantsTransparencyPolicyOrClientCert(_) => {
                Err(IoError::new(ErrorKind::Other, "missing client cert").into())
            }
            Self::ConnectorBuilder(builder) => Ok(builder.build()),
        }
    }

    pub fn load_client_certs<P: AsRef<Path>>(
        self,
        cert_path: P,
        key_path: P,
    ) -> Result<Self, IoError> {
        match self {
            Self::WantsVerifier(_) => Err(IoError::new(ErrorKind::Other, "missing verifier")),
            Self::WantsTransparencyPolicyOrClientCert(builder) => {
                Ok(builder.load_client_certs(cert_path, key_path)?.into())
            }
            Self::ConnectorBuilder(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded client cert"))
            }
        }
    }

    pub fn load_ca_cert<P: AsRef<Path>>(self, path: P) -> Result<Self, IoError> {
        match self {
            Self::WantsVerifier(builder) => Ok(builder.load_ca_cert(path)?.into()),
            Self::WantsTransparencyPolicyOrClientCert(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded ca cert"))
            }
            Self::ConnectorBuilder(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded ca cert"))
            }
        }
    }

    pub fn load_ca_cert_from_bytes(self, buffer: &[u8]) -> Result<Self, IoError> {
        match self {
            Self::WantsVerifier(builder) => Ok(builder.load_ca_cert_from_bytes(buffer)?.into()),
            Self::WantsTransparencyPolicyOrClientCert(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded ca cert"))
            }
            Self::ConnectorBuilder(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded ca cert"))
            }
        }
    }

    pub fn load_client_certs_from_bytes(
        self,
        cert_buffer: &[u8],
        key_buffer: &[u8],
    ) -> Result<Self, IoError> {
        match self {
            Self::WantsVerifier(_) => Err(IoError::new(ErrorKind::Other, "missing verifier")),
            Self::WantsTransparencyPolicyOrClientCert(builder) => Ok(builder
                .load_client_certs_from_bytes(cert_buffer, key_buffer)?
                .into()),
            Self::ConnectorBuilder(_) => {
                Err(IoError::new(ErrorKind::Other, "already loaded client cert"))
            }
        }
    }
}

impl From<ConnectorBuilderStage<WantsVerifier>> for ConnectorBuilderStages {
    fn from(builder: ConnectorBuilderStage<WantsVerifier>) -> Self {
        Self::WantsVerifier(builder)
    }
}

impl From<ConnectorBuilderStage<WantsTransparencyPolicyOrClientCert>> for ConnectorBuilderStages {
    fn from(builder: ConnectorBuilderStage<WantsTransparencyPolicyOrClientCert>) -> Self {
        Self::WantsTransparencyPolicyOrClientCert(builder)
    }
}

impl From<ConnectorBuilderWithConfig> for ConnectorBuilderStages {
    fn from(builder: ConnectorBuilderWithConfig) -> Self {
        Self::ConnectorBuilder(builder)
    }
}

//#[derive(Default)]
pub struct HyperClientBuilder(ConnectorBuilderStages);

impl ConfigBuilder for HyperClientBuilder {
    type Client = HyperClient;

    fn new() -> Self {
        Self(ConnectorBuilder::with_safe_defaults().into())
    }

    fn build(self) -> Result<Self::Client> {
        let connector = self.0.build()?;

        Ok(Client::builder()
            .executor(FluvioHyperExecutor)
            .build::<_, Body>(TlsHyperConnector::new(connector)))
    }

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> Result<Self, IoError> {
        Ok(Self(self.0.load_ca_cert(ca_path)?))
    }

    fn load_ca_cert_with_data(self, ca_data: Vec<u8>) -> Result<Self, IoError> {
        Ok(Self(self.0.load_ca_cert_from_bytes(&ca_data)?))
    }

    fn load_client_certificate_with_data(
        self,
        client_crt: Vec<u8>,
        client_key: Vec<u8>,
    ) -> Result<Self, IoError> {
        Ok(Self(
            self.0
                .load_client_certs_from_bytes(&client_crt, &client_key)?,
        ))
    }

    fn load_client_certificate<P: AsRef<Path>>(
        self,
        client_crt_path: P,
        client_key_path: P,
    ) -> Result<Self, IoError> {
        Ok(Self(
            self.0.load_client_certs(client_crt_path, client_key_path)?,
        ))
    }
}
