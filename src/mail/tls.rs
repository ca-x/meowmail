use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result};
use tokio_rustls::{
    TlsConnector,
    rustls::{ClientConfig, RootCertStore, pki_types::ServerName},
};

use super::proxy::BoxStream;

pub async fn wrap(stream: BoxStream, host: &str) -> Result<BoxStream> {
    let server_name =
        ServerName::try_from(host.to_owned()).context("mail TLS hostname is invalid")?;
    let stream = TlsConnector::from(config())
        .connect(server_name, stream)
        .await
        .context("mail TLS handshake failed")?;
    Ok(Box::new(stream))
}

fn config() -> Arc<ClientConfig> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("TLS protocol versions are valid")
                .with_root_certificates(roots)
                .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}
