use std::{
    io::BufReader,
    net::{TcpListener, TcpStream},
};

use crate::{Error, Result};

use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse, RedirectUrl,
    TokenResponse,
};
use std::io::prelude::*;

mod async_client {
    use oauth2::{HttpRequest, HttpResponse};
    use openidconnect::reqwest::Error;

    pub use reqwest;

    ///
    /// Asynchronous HTTP client.
    ///
    pub async fn async_http_client(
        request: HttpRequest,
    ) -> Result<HttpResponse, Error<reqwest::Error>> {
        let client = {
            let builder = reqwest::Client::builder();

            // Following redirects opens the client up to SSRF vulnerabilities.
            // but this is not possible to prevent on wasm targets
            #[cfg(not(target_arch = "wasm32"))]
            let builder = builder
                .danger_accept_invalid_certs(true)
                .redirect(reqwest::redirect::Policy::none());

            builder.build().map_err(Error::Reqwest)?
        };

        let mut request_builder = client
            .request(request.method, request.url.as_str())
            .body(request.body);
        for (name, value) in &request.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let request = request_builder.build().map_err(Error::Reqwest)?;

        let response = client.execute(request).await.map_err(Error::Reqwest)?;

        let status_code = response.status();
        let headers = response.headers().to_owned();
        let chunks = response.bytes().await.map_err(Error::Reqwest)?;
        Ok(HttpResponse {
            status_code,
            headers,
            body: chunks.to_vec(),
        })
    }
}
pub async fn browser_login(
    provider_url: String,
    client_id: String,
) -> Result<(String, String, String)> {
    let issuer_url = IssuerUrl::new(provider_url)?;
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url.clone(), async_client::async_http_client)
            .await
            .map_err(|_| {
                Error::AuthError("failed to discover OpenIDconnect provider metadata".to_string())
            })?;

    let server = TcpListener::bind(("127.0.0.1", 0))?;

    let client =
        CoreClient::from_provider_metadata(provider_metadata, ClientId::new(client_id), None)
            .set_redirect_uri(RedirectUrl::new(format!(
                "http://localhost:{}",
                server.local_addr()?.port()
            ))?);

    let (authorize_url, csrf_state, _nonce) = client
        .authorize_url(
            openidconnect::AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(openidconnect::Scope::new("offline_access".to_string()))
        .url();

    open::that(authorize_url.to_string())?;
    println!("Open this URL to login: \n{authorize_url}\n");

    let (mut stream, _) = server.accept()?;

    let (code, state) = extract_code_from_redirect(&stream).await;

    if csrf_state.secret() != state.secret() {
        return Err(Error::AuthError(
            "Received incorrect CSRF state".to_string(),
        ));
    }

    let message = "You are logged in!";
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
        message.len(),
        message
    );

    stream.write_all(response.as_bytes())?;

    let token_response = client
        .exchange_code(code)
        .request_async(async_client::async_http_client)
        .await
        .map_err(|_| Error::AuthError("failed to exchange code for token".to_string()))?;

    //TODO: validate claims with nonce.

    Ok((
        token_response.id_token().unwrap().to_string(),
        token_response.access_token().secret().to_string(),
        token_response.refresh_token().unwrap().secret().to_string(),
    ))
}

async fn extract_code_from_redirect(stream: &TcpStream) -> (AuthorizationCode, CsrfToken) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();

    let redirect_url = request_line.split_whitespace().nth(1).unwrap();
    let url = url::Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

    let code_pair = url
        .query_pairs()
        .find(|pair| {
            let (key, _) = pair;
            key == "code"
        })
        .unwrap();

    let (_, value) = code_pair;
    let code = AuthorizationCode::new(value.into_owned());

    let state_pair = url
        .query_pairs()
        .find(|pair| {
            let (key, _) = pair;
            key == "state"
        })
        .unwrap();

    let (_, value) = state_pair;
    let state = CsrfToken::new(value.into_owned());

    (code, state)
}
