use std::future::IntoFuture;
use tokio::sync::oneshot;
use std::net::SocketAddr;
use axum::{
    routing::get,
    Router,
    extract::{Query, State},
    response::Html,
};
use oauth2::{
    basic::BasicClient,
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct OAuth2Input {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuth2Result {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

#[tauri::command]
pub async fn oauth2_login(input: OAuth2Input) -> Result<OAuth2Result, String> {
    let client_id = ClientId::new(input.client_id);
    let client_secret = input.client_secret.map(ClientSecret::new);
    let auth_url = AuthUrl::new(input.auth_url).map_err(|e| e.to_string())?;
    let token_url = TokenUrl::new(input.token_url).map_err(|e| e.to_string())?;

    let redirect_url = "http://127.0.0.1:8484/callback";

    let client = BasicClient::new(
        client_id,
        client_secret,
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url.to_string()).unwrap());

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut auth_req = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge);

    for scope in input.scopes {
        auth_req = auth_req.add_scope(Scope::new(scope));
    }

    let (authorize_url, csrf_state) = auth_req.url();

    // Spawn an Axum server to listen for the callback
    let (tx, rx) = oneshot::channel::<AuthRequest>();
    let tx_mutex = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let app = Router::new()
        .route("/callback", get(|Query(params): Query<AuthRequest>, State(tx): State<Arc<tokio::sync::Mutex<Option<oneshot::Sender<AuthRequest>>>>>| async move {
            let mut sender = tx.lock().await;
            if let Some(s) = sender.take() {
                let _ = s.send(params);
            }
            Html("<h1>Authorization successful!</h1><p>You can close this window and return to PingZero.</p><script>window.close();</script>")
        }))
        .with_state(tx_mutex);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8484));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| e.to_string())?;
    
    // Open the browser
    let _ = webbrowser::open(authorize_url.as_str());

    // Wait for callback or timeout (e.g., 2 minutes)
    let server = axum::serve(listener, app);
    let callback_res = tokio::select! {
        _ = server.into_future() => {
            return Err("Server crashed".into());
        }
        res = rx => {
            res
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
            return Err("Authorization timed out".into());
        }
    };

    let auth_request = callback_res.map_err(|_| "Failed to receive callback".to_string())?;

    if auth_request.state != *csrf_state.secret() {
        return Err("CSRF state mismatch".into());
    }

    let token_result = client
        .exchange_code(AuthorizationCode::new(auth_request.code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Token exchange failed: {}", e))?;

    Ok(OAuth2Result {
        access_token: token_result.access_token().secret().clone(),
        refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        expires_in: token_result.expires_in().map(|d| d.as_secs()),
    })
}
