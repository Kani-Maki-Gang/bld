use anyhow::{anyhow, Result};
use awc::{Client, Connector};
use rustls::{Certificate, ClientConfig, PrivateKey, RootCertStore};
use rustls_native_certs::load_native_certs;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

pub fn load_root_certificates() -> Result<RootCertStore> {
    let mut store = RootCertStore::empty();

    for cert in load_native_certs()? {
        store.add(&Certificate(cert.0))?;
    }

    Ok(store)
}

pub fn load_server_certificate<P: AsRef<Path>>(path: &P) -> Result<Vec<Certificate>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let keys = certs(&mut reader)?
        .into_iter()
        .map(|v| Certificate(v))
        .collect();

    Ok(keys)
}

pub fn load_server_private_key<P: AsRef<Path>>(path: &P) -> Result<PrivateKey> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let keys = pkcs8_private_keys(&mut reader)?;

    let key = keys
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("private key not found"))?;

    Ok(PrivateKey(key))
}

pub fn awc_client() -> Result<Client> {
    let root_certificates = load_root_certificates()?;

    let rustls_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_certificates)
        .with_no_client_auth();

    let connector = Connector::new().rustls(Arc::new(rustls_config));

    Ok(Client::builder().connector(connector).finish())
}
