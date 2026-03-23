use askama::Template;
use axum::{
    Extension,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use snafu::ResultExt;
use tower_cookies::{Cookie, Cookies, cookie::time::Duration};

use crate::{
    Error, Result,
    dto::{Actor, OauthTokenRequestDto},
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::exchange_code_for_access_token,
};

use super::AUTH_TOKEN_COOKIE;

#[derive(Deserialize)]
pub struct AuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub description: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/oauth_error.html")]
struct OauthErrorTemplate {
    t: TemplateData,
    error_message: String,
}

pub async fn auth_callback_handler(
    Extension(csp_nonce): Extension<crate::models::CspNonce>,
    State(state): State<AppState>,
    cookies: Cookies,
    Query(query): Query<AuthCallbackQuery>,
) -> Result<Response<Body>> {
    if let Some(error) = query.error.as_deref() {
        let description = query
            .description
            .as_deref()
            .or(query.error_description.as_deref())
            .unwrap_or("No description provided.");
        let state_param = query.state.as_deref().unwrap_or("N/A");
        let message = format!(
            "error: {error}, description: {description}, state: {state_param}"
        );

        return render_oauth_error_page(&state, csp_nonce.nonce, &message, StatusCode::BAD_REQUEST);
    }

    let Some(code) = query.code else {
        return render_oauth_error_page(
            &state,
            csp_nonce.nonce,
            "Missing query parameter: code",
            StatusCode::BAD_REQUEST,
        );
    };

    let Some(state_param) = query.state else {
        return render_oauth_error_page(
            &state,
            csp_nonce.nonce,
            "Missing query parameter: state",
            StatusCode::BAD_REQUEST,
        );
    };

    let callback_url = format!("{}/auth/callback", state.config.server.public_url);
    let payload = OauthTokenRequestDto {
        client_id: state.config.auth.client_id.clone(),
        client_secret: state.config.auth.client_secret.clone(),
        code,
        state: state_param,
        redirect_uri: callback_url,
    };

    let token_response = match exchange_code_for_access_token(&state, &payload).await {
        Ok(result) => result,
        Err(err) => {
            let status = match &err {
                Error::Oauth { .. }
                | Error::InvalidOauthToken
                | Error::InvalidClient
                | Error::NoAuthToken
                | Error::InsufficientAuthScope
                | Error::RequiresAuth
                | Error::LoginRequired
                | Error::LoginFailed => StatusCode::UNAUTHORIZED,
                _ => StatusCode::BAD_GATEWAY,
            };

            return render_oauth_error_page(
                &state,
                csp_nonce.nonce,
                &err.to_string(),
                status,
            );
        }
    };

    let auth_cookie = Cookie::build((AUTH_TOKEN_COOKIE, token_response.access_token))
        .http_only(true)
        .max_age(Duration::days(7))
        .secure(state.config.server.https)
        .path("/")
        .build();
    cookies.add(auth_cookie);

    Ok(Redirect::to("/").into_response())
}

fn render_oauth_error_page(
    state: &AppState,
    nonce: String,
    message: &str,
    status: StatusCode,
) -> Result<Response<Body>> {
    let actor = Actor::default();
    let pref = Pref::new();
    let mut t = TemplateData::new(state, actor, &pref, nonce);
    t.title = String::from("OAuth Callback Error");

    let tpl = OauthErrorTemplate {
        t,
        error_message: message.to_string(),
    };

    Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}
