use core::fmt;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use turso::{Connection, Row};
use urlencoding::encode;
use validator::Validate;

use crate::Result;
use crate::db::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, row_integer, row_text,
};
use crate::db::turso_params::{integer_param, new_query_params, text_param};
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::pagination::{Paginated, PaginationParams};
use crate::utils::{IdPrefix, generate_id};

#[derive(Clone, Deserialize, Validate)]
pub struct ListImagePromptsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

impl Default for ListImagePromptsParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListImagePromptsParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(
            f,
            "page={}&per_page={}&keyword={}",
            page,
            per_page,
            encode(keyword)
        )
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewImagePromptDto {
    #[validate(length(min = 10, max = 32000))]
    pub prompt: String,

    #[validate(length(min = 1, max = 100))]
    pub model: String,

    #[validate(length(min = 1, max = 100))]
    pub background: String,

    #[validate(length(min = 1, max = 100))]
    pub moderation: String,

    #[validate(range(min = 1, max = 10))]
    pub qty: i32,

    #[validate(range(min = 1, max = 10))]
    pub output_compression: i32,

    #[validate(length(min = 1, max = 100))]
    pub output_format: String,

    #[validate(length(min = 1, max = 100))]
    pub quality: String,
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateImagePromptDto {
    #[validate(length(min = 1, max = 100))]
    pub short_title: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub status: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImagePromptDto {
    pub id: String,
    pub user_id: String,
    pub prompt: String,
    pub short_title: String,
    pub model: String,
    pub background: String,
    pub moderation: String,
    pub qty: i64,
    pub output_compression: i64,
    pub output_format: String,
    pub quality: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl FromTursoRow for ImagePromptDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            user_id: row_text(row, 1)?,
            prompt: row_text(row, 2)?,
            short_title: row_text(row, 3)?,
            model: row_text(row, 4)?,
            background: row_text(row, 5)?,
            moderation: row_text(row, 6)?,
            qty: row_integer(row, 7)?,
            output_compression: row_integer(row, 8)?,
            output_format: row_text(row, 9)?,
            quality: row_text(row, 10)?,
            status: row_text(row, 11)?,
            created_at: row_integer(row, 12)?,
            updated_at: row_integer(row, 13)?,
        })
    }
}

pub struct ImagePromptRepo {
    db_pool: Connection,
}

impl ImagePromptRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListImagePromptsParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM image_prompts
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" WHERE short_title LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(
        &self,
        params: ListImagePromptsParamsDto,
    ) -> Result<Paginated<ImagePromptDto>> {
        let mut query = r#"
            SELECT
                id,
                user_id,
                prompt,
                short_title,
                model,
                background,
                moderation,
                qty,
                output_compression,
                output_format,
                quality,
                status,
                created_at,
                updated_at
            FROM image_prompts
        "#
        .to_string();

        let mut q_params = new_query_params();
        let count_params = params.clone();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" WHERE short_title LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
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

        query.push_str(" ORDER BY id DESC LIMIT :limit OFFSET :offset");

        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<ImagePromptDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, user_id: &str, data: NewImagePromptDto) -> Result<ImagePromptDto> {
        let query = r#"
            INSERT INTO image_prompts
            (
                id,
                user_id,
                prompt,
                short_title,
                model,
                background,
                moderation,
                qty,
                output_compression,
                output_format,
                quality,
                status,
                created_at,
                updated_at
            )
            VALUES
            (
                :id,
                :user_id,
                :prompt,
                :short_title,
                :model,
                :background,
                :moderation,
                :qty,
                :output_compression,
                :output_format,
                :quality,
                :status,
                :created_at,
                :updated_at
            )
        "#;

        let id = generate_id(IdPrefix::ImagePrompt);
        let today = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();

        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":user_id", user_id.to_string()));
        q_params.push(text_param(":prompt", data.prompt.clone()));
        q_params.push(text_param(":short_title", "".to_string()));
        q_params.push(text_param(":model", data.model.clone()));
        q_params.push(text_param(":background", data.background.clone()));
        q_params.push(text_param(":moderation", data.moderation.clone()));
        q_params.push(integer_param(":qty", data.qty as i64));
        q_params.push(integer_param(
            ":output_compression",
            data.output_compression as i64,
        ));
        q_params.push(text_param(":quality", data.quality.clone()));
        q_params.push(text_param(":status", "pending".to_string()));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(ImagePromptDto {
            id,
            user_id: user_id.to_string(),
            prompt: data.prompt,
            short_title: "".to_string(),
            model: data.model,
            background: data.background,
            moderation: data.moderation,
            qty: data.qty as i64,
            output_compression: data.output_compression as i64,
            output_format: data.output_format,
            quality: data.quality,
            status: "pending".to_string(),
            created_at: today,
            updated_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<ImagePromptDto>> {
        let query = r#"
            SELECT
                id,
                user_id,
                prompt,
                short_title,
                model,
                background,
                moderation,
                qty,
                output_compression,
                output_format,
                quality,
                status,
                created_at,
                updated_at
            FROM image_prompts
            WHERE
                AND id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<ImagePromptDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, id: String, data: UpdateImagePromptDto) -> Result<bool> {
        // Do not allow empty update
        if data.short_title.is_none() && data.status.is_none() {
            return Ok(false);
        }

        let mut query = "UPDATE image_prompts SET ".to_string();
        let mut set_parts: Vec<&str> = Vec::new();
        let mut q_params = new_query_params();

        if let Some(short_title) = data.short_title {
            set_parts.push("short_title = :short_title");
            q_params.push(text_param(":short_title", short_title));
        }

        if let Some(status) = data.status {
            set_parts.push("status = :status");
            q_params.push(text_param(":status", status));
        }

        let updated_at = chrono::Utc::now().timestamp_millis();
        set_parts.push("updated_at = :updated_at");
        q_params.push(integer_param(":updated_at", updated_at));

        query.push_str(&set_parts.join(", "));
        query.push_str(" WHERE id = :id");
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }

    pub async fn delete(&self, id: String) -> Result<bool> {
        let query = r#"
            DELETE FROM image_prompts
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
