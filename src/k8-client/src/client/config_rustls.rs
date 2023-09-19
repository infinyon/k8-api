use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::Future;
use futures_util::io::{AsyncRead as StdAsyncRead, AsyncWrite as StdAsyncWrite};
use http::Uri;
use tracing::debug;

use hyper::client::connect::{Connected, Connection};
use hyper::service::Service;
use hyper::Body;
use hyper::Client;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use fluvio_future::net::TcpStream;
use fluvio_future::rust_tls::{ConnectorBuilder, DefaultClientTlsStream, TlsConnector};

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

                    let stream = connector.connect(host, tcp_stream).await.map_err(|err| {
                        IoError::new(ErrorKind::Other, format!("tls handshake: {}", err))
                    })?;
                    Ok(HyperTlsStream(stream))
                }
                scheme => Err(anyhow!("{:?}", scheme)),
            }
        })
    }
}

//#[derive(Default)]
pub struct HyperClientBuilder(ConnectorBuilder);

impl ConfigBuilder for HyperClientBuilder {
    type Client = HyperClient;

    fn new() -> Self {
        Self(ConnectorBuilder::new())
    }

    fn build(self) -> Result<Self::Client> {
        let connector = self.0.build();

        Ok(Client::builder()
            .executor(FluvioHyperExecutor)
            .build::<_, Body>(TlsHyperConnector::new(connector)))
    }

    fn load_ca_certificate(self, ca_path: impl AsRef<Path>) -> Result<Self, IoError> {
        Ok(Self(self.0.load_ca_cert(ca_path)?))
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
