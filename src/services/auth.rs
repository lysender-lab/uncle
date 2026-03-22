use prost::Message;
use reqwest::StatusCode;
use snafu::ResultExt;

use crate::dto::ActorDto;
use crate::{
    Error, Result,
    error::{HttpResponseBytesSnafu, HttpResponseParseSnafu, ProtobufDecodeSnafu},
    run::AppState,
    services::token::decode_auth_token,
};
use crate::{buffed::actor::ActorBuf, dto::Actor};

pub async fn authenticate_token(state: &AppState, token: &str) -> Result<Actor> {
    let claims = decode_auth_token(token)?;

    // Get from cache first
    if let Some(actor) = state.auth_cache.get(&claims.sub) {
        return Ok(actor);
    }

    let url = format!("{}/user/authz", &state.config.auth.api_url);
    let response = state
        .client
        .get(url.as_str())
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to process auth information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;

            let buff = ActorBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
            let actor: ActorDto = buff.try_into().map_err(|e| Error::Whatever {
                msg: format!("Unable to parse auth information: {}", e),
            })?;

            // Store to cache
            state.auth_cache.insert(
                claims.sub,
                Actor {
                    actor: Some(actor.clone()),
                },
            );

            Ok(Actor { actor: Some(actor) })
        }
        StatusCode::UNAUTHORIZED => Err(Error::LoginRequired),
        _ => Err("Unable to process auth information. Try again later.".into()),
    }
}
