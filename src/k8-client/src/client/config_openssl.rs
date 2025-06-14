use std::io::Result as IoResult;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::anyhow;
use futures_util::future::Future;
use futures_util::io::{AsyncRead as StdAsyncRead, AsyncWrite as StdAsyncWrite};
use http::Uri;
use hyper::client::connect::{Connected, Connection};
use hyper::rt::Executor;
use hyper::service::Service;
use hyper::Body;
use hyper::Client;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::debug;

use fluvio_future::{
    net::certs::CertBuilder,
    openssl::{
        TlsConnector, DefaultClientTlsStream,
        certs::{X509PemBuilder, IdentityBuilder, PrivateKeyBuilder},
    },
};
use fluvio_future::net::TcpStream;
use fluvio_future::task::spawn;

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

struct FluvioHyperExecutor;

impl<F: Future + Send + 'static> Executor<F> for FluvioHyperExecutor {
    fn execute(&self, fut: F) {
        spawn(async { drop(fut.await) });
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

#[allow(clippy::type_complexity)]
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

                    let stream = connector.connect(host, tcp_stream).await?;
                    Ok(HyperTlsStream(stream))
                }
                scheme => Err(anyhow!("{:?}", scheme)),
            }
        })
    }
}

#[derive(Default)]
pub struct HyperClientBuilder {
    ca_cert: Option<X509PemBuilder>,
    client_identity: Option<IdentityBuilder>,
}

impl ConfigBuilder for HyperClientBuilder {
    type Client = HyperClient;

    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> anyhow::Result<Self::Client> {
        let ca_cert = match self.ca_cert {
            Some(cert) => cert.build().ok(),
            None => None,
        };
        let mut connector_builder = TlsConnector::builder()?;

        if let Some(builder) = self.client_identity {
            connector_builder = connector_builder.with_identity(builder)?;
        }

        if let Some(ca_cert) = ca_cert {
            connector_builder = connector_builder.add_root_certificate(ca_cert)?;
        }

        let connector = connector_builder.build();
        Ok(Client::builder()
            .executor(FluvioHyperExecutor)
            .build::<_, Body>(TlsHyperConnector::new(connector)))
    }

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let ca_builder = X509PemBuilder::from_path(ca_path)?;
        Ok(Self {
            ca_cert: Some(ca_builder),
            client_identity: self.client_identity,
        })
    }

    fn load_ca_cert_with_data(self, ca_data: Vec<u8>) -> anyhow::Result<Self> {
        let ca_builder = X509PemBuilder::new(ca_data);

        Ok(Self {
            ca_cert: Some(ca_builder),
            client_identity: self.client_identity,
        })
    }

    fn load_client_certificate<P: AsRef<Path>>(
        self,
        client_crt_path: P,
        client_key_path: P,
    ) -> anyhow::Result<Self> {
        debug!("loading client crt from: {:#?}", client_crt_path.as_ref());
        debug!("loading client key from: {:#?}", client_key_path.as_ref());

        let identity = IdentityBuilder::from_x509(
            X509PemBuilder::from_path(client_crt_path)?,
            PrivateKeyBuilder::from_path(client_key_path)?,
        )?;

        Ok(Self {
            ca_cert: self.ca_cert,
            client_identity: Some(identity),
        })
    }

    fn load_client_certificate_with_data(
        self,
        client_crt: Vec<u8>,
        client_key: Vec<u8>,
    ) -> anyhow::Result<Self> {
        let identity = IdentityBuilder::from_x509(
            X509PemBuilder::new(client_crt),
            PrivateKeyBuilder::new(client_key),
        )?;

        Ok(Self {
            ca_cert: self.ca_cert,
            client_identity: Some(identity),
        })
    }
}
