use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{HeaderMap, HeaderValue, header, request::Parts},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cookie::{Cookie, SameSite};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{AppState, error::AppError};

const SESSION_COOKIE: &str = "meowmail_session";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

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
    csrf_token: String,
    expires_at: Instant,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    authenticated: bool,
    csrf_token: String,
    version: &'static str,
}

#[derive(Deserialize)]
struct LoginRequest {
    pin: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/session", get(session))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let record = require_session(&state, &headers)?;
    Ok(Json(SessionResponse {
        authenticated: true,
        csrf_token: record.csrf_token,
        version: env!("CARGO_PKG_VERSION"),
    }))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<SessionResponse>), AppError> {
    if input.pin.chars().count() > 128 || input.pin.chars().any(char::is_control) {
        return Err(AppError::Unauthorized);
    }
    state.sessions.check_login_allowed()?;
    let expected = Sha256::digest(state.config.pin_bytes());
    let supplied = Sha256::digest(input.pin.as_bytes());
    if expected.as_slice().ct_eq(supplied.as_slice()).unwrap_u8() != 1 {
        state.sessions.record_failed_login();
        return Err(AppError::Unauthorized);
    }

    let (token, record) = state.sessions.create();
    let secure = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("https"));
    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(secure)
        .max_age(cookie::time::Duration::seconds(SESSION_TTL.as_secs() as i64))
        .build();
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).map_err(AppError::internal)?,
    );
    Ok((
        response_headers,
        Json(SessionResponse {
            authenticated: true,
            csrf_token: record.csrf_token,
            version: env!("CARGO_PKG_VERSION"),
        }),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<HeaderMap, AppError> {
    let record = require_session(&state, &headers)?;
    require_csrf(&headers, &record)?;
    if let Some(token) = cookie_value(&headers, SESSION_COOKIE) {
        state.sessions.remove(&token);
    }
    let expired = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(cookie::time::Duration::ZERO)
        .build();
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&expired.to_string()).map_err(AppError::internal)?,
    );
    Ok(response_headers)
}

#[derive(Clone)]
pub struct AuthenticatedSession {
    pub csrf_token: String,
}

pub struct MutationSession;

impl FromRequestParts<AppState> for MutationSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        require_mutation(state, &parts.headers)?;
        Ok(Self)
    }
}

pub fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AppError> {
    let token = cookie_value(headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    state.sessions.get(&token).ok_or(AppError::Unauthorized)
}

pub fn require_mutation(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AppError> {
    let session = require_session(state, headers)?;
    require_csrf(headers, &session)?;
    Ok(session)
}

fn require_csrf(headers: &HeaderMap, session: &AuthenticatedSession) -> Result<(), AppError> {
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Csrf)?;
    if supplied
        .as_bytes()
        .ct_eq(session.csrf_token.as_bytes())
        .unwrap_u8()
        == 1
    {
        Ok(())
    } else {
        Err(AppError::Csrf)
    }
}

impl SessionStore {
    fn create(&self) -> (String, AuthenticatedSession) {
        let token = random_token();
        let csrf_token = random_token();
        let digest = token_digest(&token);
        let mut state = self.inner.lock().expect("session mutex poisoned");
        state.failed_logins.clear();
        state
            .sessions
            .retain(|_, record| record.expires_at > Instant::now());
        state.sessions.insert(
            digest,
            SessionRecord {
                csrf_token: csrf_token.clone(),
                expires_at: Instant::now() + SESSION_TTL,
            },
        );
        (token, AuthenticatedSession { csrf_token })
    }

    fn get(&self, token: &str) -> Option<AuthenticatedSession> {
        let digest = token_digest(token);
        let mut state = self.inner.lock().expect("session mutex poisoned");
        state
            .sessions
            .retain(|_, record| record.expires_at > Instant::now());
        state
            .sessions
            .get(&digest)
            .map(|record| AuthenticatedSession {
                csrf_token: record.csrf_token.clone(),
            })
    }

    fn remove(&self, token: &str) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .sessions
            .remove(&token_digest(token));
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
