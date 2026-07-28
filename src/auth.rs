use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{FromRequestParts, Query, State},
    http::{HeaderMap, HeaderValue, header, request::Parts},
    response::Redirect,
    routing::{get, post, put},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cookie::{Cookie, SameSite};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest,
};
use rand::Rng;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{
    AppState,
    config::{AuthMode, OidcConfig},
    error::AppError,
    users::{PublicUser, Role, UserRepository},
};

const SESSION_COOKIE: &str = "meowmail_session";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const OIDC_FLOW_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_OIDC_FLOWS: usize = 128;

type ConfiguredOidcClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
pub struct OidcService {
    client: ConfiguredOidcClient,
    http: reqwest::Client,
    issuer: String,
    scopes: Vec<String>,
    first_user_admin: bool,
    flows: Arc<Mutex<HashMap<String, OidcFlow>>>,
}

struct OidcFlow {
    nonce: Nonce,
    verifier: PkceCodeVerifier,
    expires_at: Instant,
}

impl OidcService {
    pub async fn discover(config: &OidcConfig) -> anyhow::Result<Self> {
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()?;
        let issuer = IssuerUrl::new(config.issuer.to_string())?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &http).await?;
        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(config.client_id.clone()),
            config
                .client_secret
                .as_ref()
                .map(|value| ClientSecret::new(value.expose_secret().to_owned())),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url.to_string())?);
        Ok(Self {
            client,
            http,
            issuer: config.issuer.to_string().trim_end_matches('/').to_owned(),
            scopes: config.scopes.clone(),
            first_user_admin: config.first_user_admin,
            flows: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn authorization_url(&self) -> url::Url {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = self.client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for scope in &self.scopes {
            if scope != "openid" {
                request = request.add_scope(Scope::new(scope.clone()));
            }
        }
        let (url, state, nonce) = request.set_pkce_challenge(challenge).url();
        let mut flows = self.flows.lock().expect("OIDC flow mutex poisoned");
        flows.retain(|_, flow| flow.expires_at > Instant::now());
        if flows.len() >= MAX_OIDC_FLOWS
            && let Some(oldest) = flows
                .iter()
                .min_by_key(|(_, flow)| flow.expires_at)
                .map(|(state, _)| state.clone())
        {
            flows.remove(&oldest);
        }
        flows.insert(
            state.secret().to_owned(),
            OidcFlow {
                nonce,
                verifier,
                expires_at: Instant::now() + OIDC_FLOW_TTL,
            },
        );
        url
    }

    fn take_flow(&self, state: &str) -> Result<OidcFlow, AppError> {
        let mut flows = self.flows.lock().expect("OIDC flow mutex poisoned");
        flows.retain(|_, flow| flow.expires_at > Instant::now());
        flows.remove(state).ok_or(AppError::Unauthorized)
    }
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<Mutex<SessionState>>,
}

#[derive(Default)]
struct SessionState {
    sessions: HashMap<[u8; 32], SessionRecord>,
    failed_logins: VecDeque<Instant>,
}

struct SessionRecord {
    user_id: uuid::Uuid,
    csrf_token: String,
    locked: bool,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub user_id: uuid::Uuid,
    pub csrf_token: String,
}

#[derive(Debug, Clone)]
struct CurrentSession {
    user_id: uuid::Uuid,
    csrf_token: String,
    locked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    authenticated: bool,
    locked: bool,
    csrf_token: String,
    version: &'static str,
    user: PublicUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthConfigResponse {
    local_enabled: bool,
    oidc_enabled: bool,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct PinRequest {
    pin: String,
}

#[derive(Deserialize)]
struct OidcCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/session", get(session))
        .route("/auth/config", get(auth_config))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/lock", post(lock))
        .route("/auth/unlock", post(unlock))
        .route("/auth/pin", put(set_pin).delete(remove_pin))
        .route("/auth/oidc/start", get(oidc_start))
        .route("/auth/oidc/callback", get(oidc_callback))
}

async fn auth_config(State(state): State<AppState>) -> Json<AuthConfigResponse> {
    Json(AuthConfigResponse {
        local_enabled: state.config.auth_mode.local_enabled(),
        oidc_enabled: state.config.auth_mode.oidc_enabled(),
    })
}

async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let current = current_session(&state, &headers)?;
    session_response(&state, current).await.map(Json)
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<SessionResponse>), AppError> {
    if !state.config.auth_mode.local_enabled()
        || input.username.len() > 128
        || input.password.len() > 4096
    {
        return Err(AppError::Unauthorized);
    }
    state.sessions.check_login_allowed()?;
    let user = UserRepository::new(state.db.clone())
        .authenticate_local(&input.username, &input.password)
        .await;
    let user = match user {
        Ok(user) => user,
        Err(AppError::Unauthorized) => {
            state.sessions.record_failed_login();
            return Err(AppError::Unauthorized);
        }
        Err(error) => return Err(error),
    };
    state.sessions.clear_failed_logins();
    let (token, current) = state.sessions.create(user.id);
    let response_headers = session_cookie_headers(&headers, token)?;
    Ok((
        response_headers,
        Json(SessionResponse {
            authenticated: true,
            locked: false,
            csrf_token: current.csrf_token,
            version: env!("CARGO_PKG_VERSION"),
            user,
        }),
    ))
}

async fn oidc_start(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let oidc = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    Ok(Redirect::temporary(oidc.authorization_url().as_str()))
}

async fn oidc_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<(HeaderMap, Redirect), AppError> {
    if query.error.is_some() {
        return Err(AppError::Unauthorized);
    }
    let oidc = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    let code = query.code.ok_or(AppError::Unauthorized)?;
    let returned_state = query.state.ok_or(AppError::Unauthorized)?;
    let flow = oidc.take_flow(&returned_state)?;
    let token_response = oidc
        .client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| AppError::Unauthorized)?
        .set_pkce_verifier(flow.verifier)
        .request_async(&oidc.http)
        .await
        .map_err(|_| AppError::Unauthorized)?;
    let id_token = token_response.id_token().ok_or(AppError::Unauthorized)?;
    let verifier = oidc.client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &flow.nonce)
        .map_err(|_| AppError::Unauthorized)?;
    if let Some(expected_hash) = claims.access_token_hash() {
        let actual_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg().map_err(|_| AppError::Unauthorized)?,
            id_token
                .signing_key(&verifier)
                .map_err(|_| AppError::Unauthorized)?,
        )
        .map_err(|_| AppError::Unauthorized)?;
        if actual_hash != *expected_hash {
            return Err(AppError::Unauthorized);
        }
    }
    let preferred = claims.preferred_username().map(|value| value.as_str());
    let email = claims.email().map(|value| value.as_str());
    let user = UserRepository::new(state.db.clone())
        .provision_oidc(
            &oidc.issuer,
            claims.subject().as_str(),
            email,
            preferred,
            oidc.first_user_admin,
        )
        .await?;
    let (token, _) = state.sessions.create(user.id);
    Ok((
        session_cookie_headers(&headers, token)?,
        Redirect::to("/mail/inbox"),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<HeaderMap, AppError> {
    let current = current_session(&state, &headers)?;
    require_csrf(&headers, &current.csrf_token)?;
    let token = cookie_value(&headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    state.sessions.remove(&token);
    expired_cookie_headers()
}

async fn lock(
    State(state): State<AppState>,
    headers: HeaderMap,
    mutation: MutationSession,
) -> Result<Json<SessionResponse>, AppError> {
    let user = UserRepository::new(state.db.clone())
        .get(mutation.0.user_id)
        .await?;
    if !user.has_pin {
        return Err(AppError::Validation(
            "set a personal PIN before locking the application".into(),
        ));
    }
    let token = cookie_value(&headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    let current = state.sessions.set_locked(&token, true)?;
    session_response(&state, current).await.map(Json)
}

async fn unlock(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PinRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let current = current_session(&state, &headers)?;
    require_csrf(&headers, &current.csrf_token)?;
    if !current.locked {
        return session_response(&state, current).await.map(Json);
    }
    state.sessions.check_login_allowed()?;
    if !UserRepository::new(state.db.clone())
        .verify_pin(current.user_id, &input.pin)
        .await?
    {
        state.sessions.record_failed_login();
        return Err(AppError::Unauthorized);
    }
    state.sessions.clear_failed_logins();
    let token = cookie_value(&headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    let current = state.sessions.set_locked(&token, false)?;
    session_response(&state, current).await.map(Json)
}

async fn set_pin(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<PinRequest>,
) -> Result<Json<PublicUser>, AppError> {
    crate::users::validate_pin(&input.pin)?;
    Ok(Json(
        UserRepository::new(state.db)
            .set_pin(mutation.0.user_id, Some(&input.pin))
            .await?,
    ))
}

async fn remove_pin(
    State(state): State<AppState>,
    mutation: MutationSession,
) -> Result<Json<PublicUser>, AppError> {
    Ok(Json(
        UserRepository::new(state.db)
            .set_pin(mutation.0.user_id, None)
            .await?,
    ))
}

async fn session_response(
    state: &AppState,
    current: CurrentSession,
) -> Result<SessionResponse, AppError> {
    Ok(SessionResponse {
        authenticated: true,
        locked: current.locked,
        csrf_token: current.csrf_token,
        version: env!("CARGO_PKG_VERSION"),
        user: UserRepository::new(state.db.clone())
            .get(current.user_id)
            .await?,
    })
}

impl FromRequestParts<AppState> for AuthenticatedSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_session(state, &parts.headers)
    }
}

pub struct MutationSession(pub AuthenticatedSession);

impl FromRequestParts<AppState> for MutationSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(require_mutation(state, &parts.headers)?))
    }
}

pub fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AppError> {
    let current = current_session(state, headers)?;
    if current.locked {
        return Err(AppError::Locked);
    }
    Ok(AuthenticatedSession {
        user_id: current.user_id,
        csrf_token: current.csrf_token,
    })
}

pub fn require_mutation(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AppError> {
    let session = require_session(state, headers)?;
    require_csrf(headers, &session.csrf_token)?;
    Ok(session)
}

fn current_session(state: &AppState, headers: &HeaderMap) -> Result<CurrentSession, AppError> {
    let token = cookie_value(headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    state.sessions.get(&token).ok_or(AppError::Unauthorized)
}

fn require_csrf(headers: &HeaderMap, expected: &str) -> Result<(), AppError> {
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Csrf)?;
    if supplied.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1 {
        Ok(())
    } else {
        Err(AppError::Csrf)
    }
}

impl SessionStore {
    fn create(&self, user_id: uuid::Uuid) -> (String, CurrentSession) {
        let token = random_token();
        let csrf_token = random_token();
        let digest = token_digest(&token);
        let mut state = self.inner.lock().expect("session mutex poisoned");
        state
            .sessions
            .retain(|_, record| record.expires_at > Instant::now());
        state.sessions.insert(
            digest,
            SessionRecord {
                user_id,
                csrf_token: csrf_token.clone(),
                locked: false,
                expires_at: Instant::now() + SESSION_TTL,
            },
        );
        (
            token,
            CurrentSession {
                user_id,
                csrf_token,
                locked: false,
            },
        )
    }

    fn get(&self, token: &str) -> Option<CurrentSession> {
        let digest = token_digest(token);
        let mut state = self.inner.lock().expect("session mutex poisoned");
        state
            .sessions
            .retain(|_, record| record.expires_at > Instant::now());
        state.sessions.get(&digest).map(|record| CurrentSession {
            user_id: record.user_id,
            csrf_token: record.csrf_token.clone(),
            locked: record.locked,
        })
    }

    fn set_locked(&self, token: &str, locked: bool) -> Result<CurrentSession, AppError> {
        let digest = token_digest(token);
        let mut state = self.inner.lock().expect("session mutex poisoned");
        let record = state
            .sessions
            .get_mut(&digest)
            .ok_or(AppError::Unauthorized)?;
        record.locked = locked;
        Ok(CurrentSession {
            user_id: record.user_id,
            csrf_token: record.csrf_token.clone(),
            locked,
        })
    }

    fn remove(&self, token: &str) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .sessions
            .remove(&token_digest(token));
    }

    pub fn revoke_user(&self, user_id: uuid::Uuid) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .sessions
            .retain(|_, record| record.user_id != user_id);
    }

    fn check_login_allowed(&self) -> Result<(), AppError> {
        let mut state = self.inner.lock().expect("session mutex poisoned");
        let cutoff = Instant::now() - Duration::from_secs(60);
        while state
            .failed_logins
            .front()
            .is_some_and(|time| *time < cutoff)
        {
            state.failed_logins.pop_front();
        }
        if state.failed_logins.len() >= 8 {
            Err(AppError::RateLimited)
        } else {
            Ok(())
        }
    }

    fn record_failed_login(&self) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .failed_logins
            .push_back(Instant::now());
    }

    fn clear_failed_logins(&self) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .failed_logins
            .clear();
    }
}

fn session_cookie_headers(headers: &HeaderMap, token: String) -> Result<HeaderMap, AppError> {
    let secure = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("https"));
    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(cookie::time::Duration::seconds(SESSION_TTL.as_secs() as i64))
        .build();
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).map_err(AppError::internal)?,
    );
    Ok(response_headers)
}

fn expired_cookie_headers() -> Result<HeaderMap, AppError> {
    let expired = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::ZERO)
        .build();
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&expired.to_string()).map_err(AppError::internal)?,
    );
    Ok(response_headers)
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub fn require_admin(user: &PublicUser) -> Result<(), AppError> {
    if user.role == Role::Admin {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn auth_mode_name(mode: AuthMode) -> &'static str {
    match mode {
        AuthMode::Local => "local",
        AuthMode::Oidc => "oidc",
        AuthMode::Hybrid => "hybrid",
    }
}
