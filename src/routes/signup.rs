use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::extract::CookieJar;
use rand::distributions::DistString;
use redis::AsyncCommands;

use crate::{
    id::Id,
    language::Language,
    model::User,
    template::BaseRenderInfo,
    util::{auth_cookie, ValidatedForm, AUTHTOKEN_TTL},
    AppState, Error, HandlerResult,
};

#[derive(serde::Serialize, Debug, Clone)]
pub struct SignUpPage {
    return_to: String,
    #[serde(flatten)]
    base: BaseRenderInfo,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct SignUpQuery {
    #[serde(default = "crate::util::default_return_to")]
    return_to: String,
}

#[derive(serde::Deserialize, garde::Validate, Clone, Debug)]
pub struct SignUpForm {
    #[garde(email, length(min = crate::util::MIN_EMAIL_LEN, max = crate::util::MAX_EMAIL_LEN))]
    email: String,
    #[garde(length(min = crate::util::MIN_USERNAME_LEN, max = crate::util::MAX_USERNAME_LEN), custom(crate::util::validate_slug))]
    username: String,
    #[garde(length(min = crate::util::MIN_PASSWORD_LEN))]
    password: String,
}

#[allow(clippy::unused_async)]
pub async fn get(
    State(state): State<AppState>,
    Query(query): Query<SignUpQuery>,
    base: BaseRenderInfo,
) -> HandlerResult {
    let ctx = SignUpPage {
        base,
        return_to: query.return_to,
    };
    state.render("signup.jinja", ctx)
}

pub async fn post(
    State(state): State<AppState>,
    cookies: CookieJar,
    ValidatedForm(form): ValidatedForm<SignUpForm>,
) -> Result<(CookieJar, Redirect), Error> {
    let password_hash_res = state
        .spawn_rayon(
            |state, password| crate::util::hash_password(password.as_bytes(), &state.argon),
            form.password,
        )
        .await?;
    let password_hash = password_hash_res?;
    let row = query!(
        "INSERT INTO users
        (username, email, password, stylesheet, flags,
            pfp, banner, biography, admin, created_at)
        VALUES ($1, $2, $3, false, 0, false, false, '', false, NOW())
        RETURNING id, username, stylesheet, pfp, banner,
        biography, admin, created_at, flags, language",
        form.username,
        form.email,
        password_hash.to_string()
    )
    .fetch_one(&state.postgres)
    .await?;
    let user = User {
        id: Id::new(row.id),
        username: row.username,
        stylesheet: row.stylesheet,
        biography: row.biography,
        pfp: row.pfp,
        banner: row.banner,
        admin: row.admin,
        created_at: row.created_at,
        flags: row.flags,
        language: row
            .language
            .map(|v| Language::from_lang_code(&v))
            .unwrap_or_default(),
    };
    let token = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 64);
    state
        .redis
        .get()
        .await?
        .set_ex(format!("token:user:{token}"), user.id.get(), AUTHTOKEN_TTL)
        .await?;
    state
        .redis
        .get()
        .await?
        .set_ex(
            format!("user:{}", user.id),
            serde_json::to_string(&user)?,
            AUTHTOKEN_TTL,
        )
        .await?;
    Ok((cookies.add(auth_cookie(token)), state.redirect("/")))
}
