use crate::Result;
use crate::db::{JobDto, ListJobsParamsDto, NewJobDto, UpdateJobDto};
use crate::pagination::Paginated;
use crate::run::AppState;

pub async fn list_jobs_svc(
    state: &AppState,
    params: ListJobsParamsDto,
) -> Result<Paginated<JobDto>> {
    state.db.jobs.list(params).await
}

pub async fn create_job_svc(state: &AppState, data: NewJobDto) -> Result<JobDto> {
    state.db.jobs.create(data).await
}

pub async fn get_job_svc(state: &AppState, id: &str) -> Result<Option<JobDto>> {
    state.db.jobs.get(id.to_string()).await
}

pub async fn update_job_svc(state: &AppState, id: &str, data: UpdateJobDto) -> Result<bool> {
    state.db.jobs.update(id.to_string(), data).await
}

pub async fn delete_job_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.jobs.delete(id.to_string()).await
}
