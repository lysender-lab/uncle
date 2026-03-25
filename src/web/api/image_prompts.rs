use axum::Json;
use axum::extract::Path;

use super::ApiMessageResponse;

pub async fn list_image_prompts_handler() -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}

pub async fn create_image_prompt_handler() -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}

pub async fn get_image_prompt_handler(Path(_id): Path<String>) -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}

pub async fn get_image_prompt_status_handler(
    Path(_id): Path<String>,
) -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}

pub async fn add_image_prompt_image_handler(Path(_id): Path<String>) -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}

pub async fn list_image_prompt_images_handler(Path(_id): Path<String>) -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}
