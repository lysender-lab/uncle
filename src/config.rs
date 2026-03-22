use serde::Deserialize;
use snafu::ResultExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::error::{ManifestParseSnafu, ManifestReadSnafu};
use crate::{Error, Result};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub db: DbConfig,
    pub auth: AuthConfig,
    pub jwt_secret: String,
    pub frontend_dir: PathBuf,
    pub openai: OpenAiConfig,
    pub aws: AwsConfig,
    pub ga_tag_id: Option<String>,
    pub assets: AssetManifest,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub https: bool,
    pub public_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig {
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub auth_url: String,
    pub api_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AwsConfig {
    pub region: String,
    pub key_id: String,
    pub secret_key: String,
    pub s3_bucket: String,
}

#[derive(Deserialize)]
struct BundleEntry {
    pub file: String,
}

type BundleConfigMap = HashMap<String, BundleEntry>;

#[derive(Clone, Deserialize)]
pub struct AssetManifest {
    pub main_css: String,
    pub main_js: String,
}

impl AssetManifest {
    pub fn build(frontend_dir: &PathBuf) -> Result<Self> {
        let filename = Path::new(frontend_dir).join("public/assets/bundles/.vite/manifest.json");
        let contents = fs::read_to_string(filename).context(ManifestReadSnafu)?;
        let config_map = serde_json::from_str::<BundleConfigMap>(contents.as_str())
            .context(ManifestParseSnafu)?;

        let main_css = config_map
            .get("bundles/main.css")
            .expect("main.css bundle is required");

        let main_js = config_map
            .get("bundles/main.js")
            .expect("main.js bundle is required");

        Ok(AssetManifest {
            main_css: format!("/assets/bundles/{}", main_css.file),
            main_js: format!("/assets/bundles/{}", main_js.file),
        })
    }
}

impl Config {
    pub fn build() -> Result<Self> {
        // Build the config from ENV vars
        let frontend_dir = PathBuf::from(required_env("FRONTEND_DIR")?);

        if !frontend_dir.exists() {
            panic!("FRONTEND_DIR does not exist");
        }

        let assets = AssetManifest::build(&frontend_dir).expect("Asset manifest should be valid");

        Ok(Config {
            server: ServerConfig {
                address: required_env("SERVER_ADDRESS")?,
                public_url: required_env("SERVER_PUBLIC_URL")?,
                https: required_env("HTTPS")? == "1",
            },
            db: DbConfig {
                filename: required_env("DB_FILENAME")?,
            },
            auth: AuthConfig {
                auth_url: required_env("AUTH_PUBLIC_BASE_URL")?,
                api_url: required_env("AUTH_API_BASE_URL")?,
                client_id: required_env("AUTH_CLIENT_ID")?,
                client_secret: required_env("AUTH_CLIENT_SECRET")?,
            },
            jwt_secret: required_env("JWT_SECRET")?,
            frontend_dir,
            openai: OpenAiConfig {
                api_key: required_env("OPENAI_API_KEY")?,
            },
            aws: AwsConfig {
                region: required_env("AWS_REGION")?,
                key_id: required_env("AWS_KEY_ID")?,
                secret_key: required_env("AWS_SECRET_KEY")?,
                s3_bucket: required_env("AWS_S3_BUCKET")?,
            },
            ga_tag_id: optional_env("GA_TAG_ID"),
            assets,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    match env::var(name) {
        Ok(val) => {
            if val.is_empty() {
                return Err(Error::Config {
                    msg: format!("{} is required.", name),
                });
            }
            Ok(val)
        }
        Err(_) => Err(Error::Config {
            msg: format!("{} is required.", name),
        }),
    }
}

fn optional_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(val) if !val.trim().is_empty() => Some(val),
        _ => None,
    }
}
