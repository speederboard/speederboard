use crate::{error::ArgonError, template::BaseRenderInfo, AppState, Error};

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use axum::{
    extract::State,
    response::{Html, Redirect},
    Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use rand::{distributions::DistString, rngs::OsRng};
use redis::AsyncCommands;
use tera::Context;

#[derive(serde::Serialize)]
pub struct SignUpPage<'a> {
    #[serde(flatten)]
    core: BaseRenderInfo<'a>,
}

#[derive(serde::Serialize)]
pub struct SignUpForm<'a> {
    username: String,
    email: String,
    #[serde(flatten)]
    core: BaseRenderInfo<'a>,
}

#[derive(serde::Deserialize)]
pub struct SignUpFormData {
    email: String,
    username: String,
    password: String,
}

#[allow(clippy::unused_async)]
pub async fn page(State(state): State<AppState>) -> Result<Html<String>, Error> {
    let ctx = SignUpPage {
        core: BaseRenderInfo::new(&state.config.root_url),
    };
    let context_ser = Context::from_serialize(ctx)?;
    Ok(Html(state.tera.render("signup.jinja", &context_ser)?))
}

pub async fn form(
    State(state): State<AppState>,
    cookies: CookieJar,
    Form(form): Form<SignUpFormData>,
) -> Result<(CookieJar, Redirect), Error> {
    if form.username.len() > 128 {
        return Err(Error::FormValidation(
            "username",
            "be less then 128 characters",
        ));
    }
    if form.email.len() > 255 {
        return Err(Error::FormValidation(
            "email",
            "be less then 255 characters",
        ));
    }
    let password_hash = state
        .spawn_rayon(move |state| hash_password(form.password.as_bytes(), &state.argon))
        .await??;
    let id = query!(
        "INSERT INTO users (username, email, password) VALUES ($1, $2, $3) RETURNING id",
        form.username,
        form.email,
        password_hash.to_string()
    )
    .fetch_one(&state.postgres)
    .await?
    .id;
    let token = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 64);
    state.redis.get().await?.set_ex(&token, id, 86_400).await?;
    Ok((
        cookies.add(Cookie::new("auth", token)),
        Redirect::to(&state.config.root_url),
    ))
}

fn hash_password(password: &[u8], argon: &Argon2) -> Result<String, ArgonError> {
    let salt = SaltString::generate(&mut OsRng);
    argon
        .hash_password(password, &salt)
        .map_err(Into::into)
        .map(|v| v.to_string())
}
