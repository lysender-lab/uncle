use core::fmt;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use turso::{Connection, Row};
use urlencoding::encode;
use validator::Validate;

use crate::Result;
use crate::db::turso_decode::opt_row_text;
use crate::db::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, row_integer, row_text,
};
use crate::db::turso_params::{integer_param, new_query_params, opt_text_param, text_param};
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::pagination::{Paginated, PaginationParams};
use crate::utils::{IdPrefix, generate_id};

#[derive(Clone, Deserialize, Validate)]
pub struct ListJobsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub status: Option<String>,
}

impl Default for ListJobsParamsDto {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(10),
            status: None,
        }
    }
}

impl fmt::Display for ListJobsParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.status.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let status = self.status.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(
            f,
            "page={}&per_page={}&status={}",
            page,
            per_page,
            encode(status)
        )
    }
}

#[derive(Clone, Deserialize)]
pub struct NewJobDto {
    pub job_type: String,
    pub prompt_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct UpdateJobDto {
    pub status: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct JobDto {
    pub id: String,
    pub job_type: String,
    pub prompt_id: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl FromTursoRow for JobDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            job_type: row_text(row, 1)?,
            prompt_id: opt_row_text(row, 2)?,
            status: row_text(row, 11)?,
            created_at: row_integer(row, 12)?,
            updated_at: row_integer(row, 13)?,
        })
    }
}

pub struct JobRepo {
    db_pool: Connection,
}

impl JobRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListJobsParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM jobs
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(status) = params.status
            && !status.is_empty()
        {
            query.push_str(" WHERE status = :status");
            q_params.push(text_param(":status", status));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(&self, params: ListJobsParamsDto) -> Result<Paginated<JobDto>> {
        let mut query = r#"
            SELECT
                id,
                job_type,
                prompt_id,
                status,
                created_at,
                updated_at
            FROM jobs
        "#
        .to_string();

        let mut q_params = new_query_params();
        let count_params = params.clone();

        if let Some(status) = params.status
            && !status.is_empty()
        {
            query.push_str(" WHERE status = :status");
            q_params.push(text_param(":status", status));
        }

        let total_records = self.listing_count(count_params).await?;

        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        // Do not query if we already know there are no records
        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY id ASC LIMIT :limit OFFSET :offset");

        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<JobDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewJobDto) -> Result<JobDto> {
        let query = r#"
            INSERT INTO jobs
            (
                id,
                job_type,
                prompt_id,
                status,
                created_at,
                updated_at
            )
            VALUES
            (
                :id,
                :job_type,
                :prompt_id,
                :status,
                :created_at,
                :updated_at
            )
        "#;

        let id = generate_id(IdPrefix::Job);
        let today = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();

        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":job_type", data.job_type.clone()));
        q_params.push(opt_text_param(":prompt_id", data.prompt_id.clone()));
        q_params.push(text_param(":status", "pending".to_string()));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(JobDto {
            id,
            job_type: data.job_type,
            prompt_id: data.prompt_id,
            status: "pending".to_string(),
            created_at: today,
            updated_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<JobDto>> {
        let query = r#"
            SELECT
                id,
                job_type,
                prompt_id,
                status,
                created_at,
                updated_at
            FROM jobs
            WHERE
                AND id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<JobDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, id: String, data: UpdateJobDto) -> Result<bool> {
        let query = r#"
            UPDATE jobs
            SET
                status = :status,
                updated_at = :updated_at
            WHERE
                id = :id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":status", data.status));

        let updated_at = chrono::Utc::now().timestamp_millis();
        q_params.push(integer_param(":updated_at", updated_at));
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }

    pub async fn delete(&self, id: String) -> Result<bool> {
        let query = r#"
            DELETE FROM jobs
            WHERE
                id = :id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }
}
