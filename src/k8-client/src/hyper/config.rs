use std::fs::File;
use std::io::BufReader;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Read;
use std::path::Path;

use hyper::client::connect::HttpConnector;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use rustls::internal::pemfile::certs;
use rustls::internal::pemfile::rsa_private_keys;
use rustls::Certificate;
use rustls::ClientConfig;
use rustls::PrivateKey;
use tracing::debug;

use crate::cert::ConfigBuilder;
use crate::ClientConfigBuilder;
use crate::ClientError;

pub type HyperBuilder = ClientConfigBuilder<HyperClientBuilder>;
pub type HyperHttpsClient = Client<HttpsConnector<HttpConnector>>;

/*
struct NoVerifier {}

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _roots: &RootCertStore,
        _presented_certs: &[Certificate],
        dns_name: DNSNameRef,
        _ocsp_response: &[u8],
    ) -> Result<ServerCertVerified, TLSError> {

        trace!("decoding dns: {:#?}",dns_name);
        Ok(ServerCertVerified::assertion())
    }
}

*/

pub struct HyperClientBuilder(ClientConfig);

impl ConfigBuilder for HyperClientBuilder {
    type Client = HyperHttpsClient;

    fn new() -> Self {
        Self(ClientConfig::new())
    }

    fn build(self) -> Result<Self::Client, ClientError> {
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        let connector = HttpsConnector::from((http, self.0));
        Ok(Client::builder().build(connector))
    }

    fn load_ca_certificate<P>(mut self, ca_path: P) -> Result<Self, IoError>
    where
        P: AsRef<Path>,
    {
        let f = File::open(ca_path)?;
        let mut rd = BufReader::new(f);

        if let Err(_) = &mut self.0.root_store.add_pem_file(&mut rd) {
            Err(IoError::new(
                ErrorKind::InvalidInput,
                "problem loading root certificate".to_owned(),
            ))
        } else {
            Ok(self)
        }
    }

    fn load_client_certificate<P>(
        mut self,
        client_crt_path: P,
        client_key_path: P,
    ) -> Result<Self, IoError>
    where
        P: AsRef<Path>,
    {
        let client_certs = retrieve_cert_from_file(&client_crt_path)?;
        debug!("retrieved client certs");
        let mut private_keys = retrieve_private_key(client_key_path)?;
        debug!("retrieved client private key");

        if private_keys.len() == 0 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "private key not founded",
            ));
        }

        &mut self
            .0
            .set_single_client_cert(client_certs, private_keys.remove(0));

        Ok(self)
    }
}

fn retrieve_cert<R>(reader: R) -> Result<Vec<Certificate>, IoError>
where
    R: Read,
{
    let mut reader = BufReader::new(reader);
    certs(&mut reader).map_err(|_| IoError::new(ErrorKind::Other, format!("no cert found")))
}

fn retrieve_cert_from_file<P>(file_path: P) -> Result<Vec<Certificate>, IoError>
where
    P: AsRef<Path>,
{
    let file = File::open(file_path)?;
    retrieve_cert(file)
}

fn retrieve_private_key<P>(filename: P) -> Result<Vec<PrivateKey>, IoError>
where
    P: AsRef<Path>,
{
    let key_file = File::open(filename)?;
    let mut reader = BufReader::new(key_file);
    rsa_private_keys(&mut reader)
        .map_err(|_| IoError::new(ErrorKind::InvalidData, "private key not founded"))
}
