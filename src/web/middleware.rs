use axum::{
    Extension,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use urlencoding::encode;

use crate::dto::Actor;

use crate::{
    Error, Result,
    ctx::Ctx,
    error::ErrorInfo,
    models::{CspNonce, Pref},
    run::AppState,
    services::authenticate_token,
    web::handle_error,
};

use super::{AUTH_TOKEN_COOKIE, THEME_COOKIE};

/// Generates a nonce value for csp and make it available in request and response extensions
pub async fn csp_nonce_middleware(mut req: Request, next: Next) -> Response {
    let csp_nonce = CspNonce::new();
    req.extensions_mut().insert(csp_nonce.clone());

    let mut response = next.run(req).await;
    response.extensions_mut().insert(csp_nonce);
    response
}

/// Validates auth token but does not require its validity
pub async fn auth_middleware(
    csp_nonce: Extension<CspNonce>,
    pref: Extension<Pref>,
    state: State<AppState>,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let token = cookies
        .get(AUTH_TOKEN_COOKIE)
        .map(|c| c.value().to_string());

    let full_page = req.headers().get("HX-Request").is_none();

    // Allow ctx to be always present
    let mut ctx: Ctx = Ctx::new(Actor::default(), None);

    if let Some(token) = token {
        // Validate token
        let result = authenticate_token(&state, &token).await;

        match result {
            Ok(actor) => {
                ctx = Ctx::new(actor, Some(token));
            }
            Err(err) => match err {
                Error::LoginRequired => {
                    // Allow passing through
                }
                _ => {
                    return handle_error(
                        &state,
                        Actor::default(),
                        &pref,
                        csp_nonce.nonce.clone(),
                        ErrorInfo::from(&err),
                        full_page,
                    );
                }
            },
        };
    }

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

pub async fn require_auth_middleware(
    ctx: Extension<Ctx>,
    state: State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let full_page = req.headers().get("HX-Request").is_none();

    if !ctx.actor.has_auth_scope() {
        if full_page {
            let callback_url = state.config.server.public_url.clone();
            let scope = encode("auth oauth");
            // Generate current millis as state
            let oauth_state = Utc::now().timestamp_millis();
            let authorize_url = format!(
                "{}/oauth/authorize?client_id={}&scope={}&state={}&redirect_uri={}",
                state.config.auth.auth_url,
                state.config.auth.client_id,
                scope,
                oauth_state,
                callback_url
            );
            return Ok(Redirect::to(&authorize_url).into_response());
        } else {
            return Err(Error::LoginRequired);
        }
    }

    Ok(next.run(req).await)
}

pub async fn pref_middleware(cookies: CookieJar, mut req: Request, next: Next) -> Response {
    let mut pref = Pref::new();
    let theme = cookies.get(THEME_COOKIE).map(|c| c.value().to_string());

    if let Some(theme) = theme {
        let t = theme.as_str();
        if t == "dark" || t == "light" {
            pref.theme = theme;
        }
    }

    req.extensions_mut().insert(pref);
    next.run(req).await
}
