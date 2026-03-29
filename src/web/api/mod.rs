mod image_prompts;
mod images;

use axum::Router;
use axum::routing::{get, post};
use serde::Serialize;

use crate::run::AppState;

pub use image_prompts::*;
pub use images::*;

#[derive(Serialize)]
pub struct ApiMessageResponse {
    pub status_code: u16,
    pub message: String,
}

impl ApiMessageResponse {
    pub fn ok() -> Self {
        Self {
            status_code: 200,
            message: String::from("OK"),
        }
    }
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/image-prompts", get(list_image_prompts_handler))
        .route("/image-prompts", post(create_image_prompt_handler))
        .route("/image-prompts/{id}", get(get_image_prompt_handler))
        .route(
            "/image-prompts/{id}/status",
            get(get_image_prompt_status_handler),
        )
        .route("/images/upload-urls", post(get_image_upload_urls_handler))
        .route(
            "/image-prompts/{id}/images",
            post(add_image_prompt_image_handler).get(list_image_prompt_images_handler),
        )
}
