use anyhow::{Result, anyhow};
use rustls::RootCertStore;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_native_certs::load_native_certs;
use std::path::Path;

pub fn load_root_certificates() -> Result<RootCertStore> {
    let mut store = RootCertStore::empty();

    let result = load_native_certs();
    let (added, _ignored) = store.add_parsable_certificates(result.certs);
    if added == 0 {
        return Err(anyhow!("no native root certificates could be loaded"));
    }

    Ok(store)
}

pub fn load_server_certificate<P: AsRef<Path>>(path: &P) -> Result<Vec<CertificateDer<'static>>> {
    let certs = CertificateDer::pem_file_iter(path)?.collect::<Result<Vec<_>, _>>()?;

    Ok(certs)
}

pub fn load_server_private_key<P: AsRef<Path>>(path: &P) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(path).map_err(|e| anyhow!("private key not found: {e}"))
}
