use crate::{template::BaseRenderInfo, AppState, HandlerResult};
use axum::{extract::State, http::StatusCode};

pub mod admin;
pub mod forum;
pub mod game;
pub mod index;
pub mod login;
pub mod settings;
pub mod signup;
pub mod user;

#[allow(clippy::unused_async)]
pub async fn notfound_handler(
    State(state): State<AppState>,
    base: BaseRenderInfo,
) -> (StatusCode, HandlerResult) {
    notfound(&state, base)
}

pub fn notfound(state: &AppState, base: BaseRenderInfo) -> (StatusCode, HandlerResult) {
    (StatusCode::NOT_FOUND, state.render("404.jinja", base))
}
