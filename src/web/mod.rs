mod auth;
mod error;
mod index;
mod logout;
mod middleware;
mod policies;
mod pref;
mod routes;
mod security_headers;

pub const AUTH_TOKEN_COOKIE: &str = "uncle_auth_token";
pub const THEME_COOKIE: &str = "uncle_theme";

pub use auth::*;
pub use error::*;
pub use index::*;
pub use logout::*;
pub use policies::*;
pub use pref::*;
pub use routes::*;
