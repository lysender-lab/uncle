use axum::Json;

use super::ApiMessageResponse;

pub async fn get_image_upload_urls_handler() -> Json<ApiMessageResponse> {
    Json(ApiMessageResponse::ok())
}
