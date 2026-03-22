use crate::Result;
use crate::db::{
    ImagePromptDto, ListImagePromptsParamsDto, NewImagePromptDto, UpdateImagePromptDto,
};
use crate::pagination::Paginated;
use crate::run::AppState;

pub async fn list_image_prompts_svc(
    state: &AppState,
    params: ListImagePromptsParamsDto,
) -> Result<Paginated<ImagePromptDto>> {
    state.db.image_prompts.list(params).await
}

pub async fn create_image_prompt(
    state: &AppState,
    user_id: &str,
    data: NewImagePromptDto,
) -> Result<ImagePromptDto> {
    state.db.image_prompts.create(user_id, data).await
}

pub async fn get_image_prompt_svc(state: &AppState, id: &str) -> Result<Option<ImagePromptDto>> {
    state.db.image_prompts.get(id.to_string()).await
}

pub async fn update_image_prompt_svc(
    state: &AppState,
    id: &str,
    data: UpdateImagePromptDto,
) -> Result<bool> {
    state.db.image_prompts.update(id.to_string(), data).await
}
