use snafu::ResultExt;
use turso::{Builder, Connection};

use crate::db::image::ImageRepo;
use crate::db::image_prompt::ImagePromptRepo;
use crate::db::job::JobRepo;
use crate::error::{DbBuilderSnafu, DbConnectSnafu};

use crate::Result;

pub async fn create_db_pool(filename: &str) -> Result<Connection> {
    let db = Builder::new_local(filename)
        .build()
        .await
        .context(DbBuilderSnafu)?;
    let conn = db.connect().context(DbConnectSnafu)?;

    Ok(conn)
}

pub struct DbMapper {
    pub image_prompts: ImagePromptRepo,
    pub images: ImageRepo,
    pub jobs: JobRepo,
}

pub async fn create_db_mapper(filename: &str) -> Result<DbMapper> {
    let pool = create_db_pool(filename).await?;
    Ok(DbMapper {
        image_prompts: ImagePromptRepo::new(pool.clone()),
        images: ImageRepo::new(pool.clone()),
        jobs: JobRepo::new(pool),
    })
}
