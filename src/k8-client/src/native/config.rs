
use std::io::Error as IoError;
use std::path::Path;

use log::debug;
use isahc::HttpClient;
use isahc::HttpClientBuilder;
use isahc::config::ClientCertificate;
use isahc::config::PrivateKey;
use isahc::config::CaCertificate;


use crate::cert::ConfigBuilder;
use crate::ClientError;
use crate::ClientConfigBuilder;

pub type IsahcBuilder = ClientConfigBuilder<IsahcConfigBuilder>;

/// load client certificate 
fn load_client_certificate<P>(client_crt_path: P,client_key_path: P) -> ClientCertificate 
    where P: AsRef<Path>
{
    ClientCertificate::pem_file(
        client_crt_path.as_ref().to_owned(),
        PrivateKey::pem_file(client_key_path.as_ref().to_owned(), String::from("")),
    )
}

fn load_ca_certificate<P>(ca_path: P) -> CaCertificate 
    where P: AsRef<Path>
{
    CaCertificate::file(ca_path.as_ref().to_owned())
}



pub struct IsahcConfigBuilder(HttpClientBuilder);


impl ConfigBuilder for IsahcConfigBuilder {

    type Client = HttpClient;


    fn new() -> Self {
        Self(HttpClientBuilder::new())
    }

    fn build(self) -> Result<Self::Client,ClientError> {
        self.0.build().map_err(|err| err.into())
    }

    fn load_ca_certificate<P>(self,ca_path: P)  -> Result<Self,IoError>
        where P: AsRef<Path> {

        let ca_certificate = load_ca_certificate(ca_path);

        debug!("retrieved CA certificate");
        let inner = self.0.ssl_ca_certificate(ca_certificate);

        Ok(Self(inner))
    }

    fn load_client_certificate<P>(self,client_crt_path: P,client_key_path: P) -> Result<Self,IoError>
        where P: AsRef<Path> {


        let client_certificate = load_client_certificate(client_crt_path,client_key_path);
        debug!("retrieved client certs from kubeconfig");
        let inner = self.0.ssl_client_certificate(client_certificate);
        Ok(Self(inner))
    }


}





