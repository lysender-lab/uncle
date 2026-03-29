mod auth;
mod image;
mod image_prompt;
mod job;
mod oauth;
mod token;

use reqwest::StatusCode;
use snafu::ResultExt;

use crate::{
    Error, Result,
    dto::ErrorMessageDto,
    error::{ErrorResponse, HttpResponseBytesSnafu, HttpResponseParseSnafu},
};

pub use auth::*;
pub use image::*;
pub use image_prompt::*;
pub use job::*;
pub use oauth::*;
pub use token::*;

pub async fn handle_response_error(
    response: reqwest::Response,
    resource: &str,
    default_error: Error,
) -> Error {
    // Assumes that ok responses are already handled
    let status = response.status();
    let message_res = parse_response_error(response).await;
    match message_res {
        Ok(msg) => match status {
            StatusCode::BAD_REQUEST => Error::BadRequest { msg },
            StatusCode::UNAUTHORIZED => Error::LoginRequired,
            StatusCode::FORBIDDEN => Error::Forbidden {
                msg: format!("You have no permissions to view {}", resource),
            },
            StatusCode::NOT_FOUND => default_error,
            _ => Error::Service {
                msg: "Service error. Try again later.".to_string(),
            },
        },
        Err(err) => err,
    }
}

pub async fn parse_response_error(response: reqwest::Response) -> Result<String> {
    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        let error = response
            .json::<ErrorMessageDto>()
            .await
            .context(HttpResponseParseSnafu {
                msg: "Unable to parse JSON error response.".to_string(),
            })?;
        return Ok(error.message);
    }

    let text = response.text().await.context(HttpResponseParseSnafu {
        msg: "Unable to parse service error response.".to_string(),
    })?;

    if let Ok(error) = serde_json::from_str::<ErrorMessageDto>(&text) {
        return Ok(error.message);
    }

    if !text.is_empty() {
        return Ok(text);
    }

    Err(Error::Service {
        msg: "Unable to parse service error response".to_string(),
    })
}
