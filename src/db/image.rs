use core::fmt;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use turso::{Connection, Row};
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
pub struct ListImagesParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,
}

impl Default for ListImagesParamsDto {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListImagesParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(f, "page={}&per_page={}", page, per_page,)
    }
}

#[derive(Clone, Deserialize)]
pub struct NewImageDto {
    pub category: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_path: String,
    pub dimensions: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageDto {
    pub id: String,
    pub user_id: String,
    pub prompt_id: String,
    pub category: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_path: String,
    pub dimensions: String,
    pub created_at: i64,
}

impl FromTursoRow for ImageDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            user_id: row_text(row, 1)?,
            prompt_id: row_text(row, 2)?,
            category: row_text(row, 3)?,
            filename: row_text(row, 4)?,
            file_type: row_text(row, 5)?,
            file_size: row_integer(row, 6)?,
            file_path: row_text(row, 7)?,
            dimensions: row_text(row, 8)?,
            created_at: row_integer(row, 9)?,
        })
    }
}

pub struct ImageRepo {
    db_pool: Connection,
}

impl ImageRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, user_id: &str, prompt_id: &str) -> Result<i64> {
        let query = r#"
            SELECT COUNT(*) AS total_count
            FROM images
            WHERE user_id = :user_id AND prompt_id = :prompt_id
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":user_id", user_id.to_string()));
        q_params.push(text_param(":prompt_id", prompt_id.to_string()));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(
        &self,
        user_id: &str,
        prompt_id: &str,
        params: ListImagesParamsDto,
    ) -> Result<Paginated<ImageDto>> {
        let mut query = r#"
            SELECT
                id,
                user_id,
                prompt_id,
                category,
                filename,
                file_type,
                file_size,
                file_path,
                dimensions,
                created_at
            FROM images
            WHERE user_id = :user_id AND prompt_id = :prompt_id
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":user_id", user_id.to_string()));
        q_params.push(text_param(":prompt_id", prompt_id.to_string()));

        let total_records = self.listing_count(user_id, prompt_id).await?;

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
        let items: Vec<ImageDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(
        &self,
        user_id: &str,
        prompt_id: &str,
        data: NewImageDto,
    ) -> Result<ImageDto> {
        let query = r#"
            INSERT INTO images
            (
                id,
                user_id,
                prompt_id,
                category,
                filename,
                file_type,
                file_size,
                file_path,
                dimensions,
                created_at
            )
            VALUES
            (
                :id,
                :user_id,
                :prompt_id,
                :category,
                :filename,
                :file_type,
                :file_size,
                :file_path,
                :dimensions,
                :created_at
            )
        "#;

        let id = generate_id(IdPrefix::Image);
        let today = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();

        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":user_id", user_id.to_string()));
        q_params.push(text_param(":prompt_id", prompt_id.to_string()));
        q_params.push(text_param(":category", data.category.clone()));
        q_params.push(text_param(":filename", data.filename.clone()));
        q_params.push(text_param(":file_type", data.file_type.clone()));
        q_params.push(integer_param(":file_size", data.file_size));
        q_params.push(text_param(":file_path", data.file_path.clone()));
        q_params.push(text_param(":dimensions", data.dimensions.clone()));
        q_params.push(integer_param(":created_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(ImageDto {
            id,
            user_id: user_id.to_string(),
            prompt_id: prompt_id.to_string(),
            category: data.category,
            filename: data.filename,
            file_type: data.file_type,
            file_size: data.file_size,
            file_path: data.file_path,
            dimensions: data.dimensions,
            created_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<ImageDto>> {
        let query = r#"
            SELECT
                id,
                user_id,
                prompt_id,
                category,
                filename,
                file_type,
                file_size,
                file_path,
                dimensions,
                created_at
            FROM images
            WHERE
                AND id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<ImageDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn delete(&self, id: String) -> Result<bool> {
        let query = r#"
            DELETE FROM images
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
