use snafu::ResultExt;

use crate::Result;
use crate::db::{ImageDto, ListImagesParamsDto, NewImageDto};
use crate::pagination::Paginated;
use crate::run::AppState;

pub async fn list_images_svc(
    state: &AppState,
    user_id: &str,
    prompt_id: &str,
    params: ListImagesParamsDto,
) -> Result<Paginated<ImageDto>> {
    state.db.images.list(user_id, prompt_id, params).await
}

pub async fn create_image_svc(
    state: &AppState,
    user_id: &str,
    prompt_id: &str,
    data: NewImageDto,
) -> Result<ImageDto> {
    state.db.images.create(user_id, prompt_id, data).await
}

pub async fn get_image_svc(state: &AppState, id: &str) -> Result<Option<ImageDto>> {
    state.db.images.get(id.to_string()).await
}

pub async fn delete_image_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.images.delete(id.to_string()).await
}
