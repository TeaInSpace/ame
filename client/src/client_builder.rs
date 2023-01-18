use crate::Result;

use hyper::{client::HttpConnector, Uri};

use tokio_rustls::rustls::{client::ServerCertVerifier, ClientConfig};

use tower::util::BoxService;
use tower_http::auth::AddAuthorizationLayer;

use crate::ame_service_client::AmeServiceClient;

struct Verifier;

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::Certificate,
        _intermediates: &[tokio_rustls::rustls::Certificate],
        _server_name: &tokio_rustls::rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<
        tokio_rustls::rustls::client::ServerCertVerified,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::ServerCertVerified::assertion())
    }
}

pub struct AmeServiceClientCfg {
    pub disable_tls_cert_check: bool,
    pub endpoint: Uri,
    pub id_token: Option<String>,
}

pub type AmeClient = AmeServiceClient<
    tower::buffer::Buffer<
        BoxService<
            http::Request<tonic::body::BoxBody>,
            http::Response<tonic::transport::Body>,
            hyper::Error,
        >,
        http::Request<tonic::body::BoxBody>,
    >,
>;

pub async fn build_ame_client(cfg: AmeServiceClientCfg) -> Result<AmeClient> {
    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("missing cerst") {
        roots
            .add(&tokio_rustls::rustls::Certificate(cert.0))
            .unwrap()
    }

    let mut tls = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    tls.dangerous()
        .set_certificate_verifier(std::sync::Arc::new(Verifier {}));

    let mut http = HttpConnector::new();
    http.enforce_http(false);

    let id_token = if let Some(id_token) = cfg.id_token {
        id_token
    } else {
        "".to_string()
    };

    let connector = tower::ServiceBuilder::new()
        .layer_fn(move |s| {
            let tls = tls.clone();

            hyper_rustls::HttpsConnectorBuilder::new()
                .with_tls_config(tls)
                .https_or_http()
                .enable_http2()
                .wrap_connector(s)
        })
        .service(http.clone());

    let res = hyper::Client::builder().http2_only(true).build(connector);
    let svc = tower::ServiceBuilder::new()
        .layer(tower::buffer::BufferLayer::new(1024))
        .layer(BoxService::layer())
        .layer(AddAuthorizationLayer::bearer(&id_token))
        .map_request(move |mut req: http::Request<tonic::body::BoxBody>| {
            let uri = Uri::builder()
                .scheme(cfg.endpoint.scheme().unwrap().clone())
                .authority(cfg.endpoint.authority().unwrap().clone())
                .path_and_query(req.uri().path_and_query().unwrap().clone())
                .build()
                .unwrap();

            *req.uri_mut() = uri;

            req
        })
        .service(res);

    Ok(AmeServiceClient::new(svc))
}
