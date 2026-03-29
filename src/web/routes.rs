use axum::extract::State;
use axum::handler::HandlerWithoutStateExt;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, get_service, post};
use axum::{Extension, Json, Router, middleware};
use reqwest::StatusCode;
use std::path::Path;
use std::sync::Arc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::services::{ServeDir, ServeFile};
use tracing::error;

use crate::ctx::Ctx;
use crate::error::{ErrorInfo, ErrorResponse};
use crate::models::{CspNonce, Pref};
use crate::run::AppState;
use crate::web::{api::api_routes, auth_callback_handler, error_handler, index_handler};

use super::middleware::{
    ApiRequest, auth_middleware, csp_nonce_middleware, pref_middleware, require_auth_middleware,
};
use super::security_headers::add_security_headers;
use super::{dark_theme_handler, handle_error, light_theme_handler};

pub fn all_routes(state: AppState, frontend_dir: &Path) -> Router {
    Router::new()
        .merge(public_routes(state.clone()))
        .merge(private_routes(state.clone()))
        .merge(assets_routes(frontend_dir))
        .layer(middleware::from_fn(add_security_headers))
        .layer(middleware::from_fn(csp_nonce_middleware))
        .fallback(any(error_handler).with_state(state))
}

pub fn public_routes(state: AppState) -> Router {
    Router::new()
        .route("/auth/callback", get(auth_callback_handler))
        .with_state(state)
}

pub fn assets_routes(dir: &Path) -> Router {
    let target_dir = dir.join("public");
    Router::new()
        .route(
            "/manifest.json",
            get_service(ServeFile::new(target_dir.join("manifest.json"))),
        )
        .route(
            "/favicon.ico",
            get_service(ServeFile::new(target_dir.join("favicon.ico"))),
        )
        .nest_service(
            "/assets",
            get_service(
                ServeDir::new(target_dir.join("assets"))
                    .not_found_service(file_not_found.into_service()),
            ),
        )
}

async fn file_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "File not found")
}

pub fn private_routes(state: AppState) -> Router {
    // Rate limiter: 120 requests per minute per IP for authenticated routes
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(120)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to create default rate limiter config"),
    );

    Router::new()
        .route("/", get(index_handler))
        .nest("/api", api_routes())
        .route("/prefs/theme/light", post(light_theme_handler))
        .route("/prefs/theme/dark", post(dark_theme_handler))
        .layer(GovernorLayer::new(governor_config))
        .layer(middleware::map_response_with_state(
            state.clone(),
            response_mapper,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(pref_middleware))
        .with_state(state)
}

async fn response_mapper(
    State(state): State<AppState>,
    Extension(csp_nonce): Extension<CspNonce>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(api_request): Extension<ApiRequest>,
    headers: HeaderMap,
    mut res: Response,
) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            error!("{}", e.message);
        }

        let full_page = headers.get("HX-Request").is_none();
        if api_request.0 {
            return (
                e.status_code,
                Json(ErrorResponse {
                    status_code: e.status_code.as_u16(),
                    message: e.message.clone(),
                }),
            )
                .into_response();
        }

        return handle_error(
            &state,
            ctx.actor.clone(),
            &pref,
            csp_nonce.nonce,
            e.clone(),
            full_page,
        );
    }

    let content_type_missing = !res.headers().contains_key(header::CONTENT_TYPE);
    if content_type_missing {
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            "text/html; charset=utf-8".parse().unwrap(),
        );
    }
    res
}
