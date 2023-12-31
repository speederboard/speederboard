use axum::{
    extract::{Path, State},
    response::Redirect,
};

use crate::{
    id::{CategoryMarker, Id, RunMarker},
    model::{Member, Permissions, ResolvedRun, User},
    routes::game::run::RunPage,
    template::BaseRenderInfo,
    util::game_n_member,
    AppState, Error, HandlerResult,
};

pub async fn fetch_review(
    State(state): State<AppState>,
    Path((game_slug, category_id, run_id)): Path<(String, Id<CategoryMarker>, Id<RunMarker>)>,
    user: User,
    base: BaseRenderInfo,
) -> HandlerResult {
    let run = ResolvedRun::from_db(&state, run_id)
        .await?
        .ok_or(Error::NotFound)?;
    if run.game.slug != game_slug || run.category.id != category_id {
        return Err(Error::NotFound);
    }
    let member = Member::from_db(&state, user.id, run.game.id)
        .await?
        .ok_or(Error::InsufficientPermissions)?;
    drop(user);
    if !member.perms.contains(Permissions::VERIFY_RUNS) {
        return Err(Error::InsufficientPermissions);
    }
    let ctx = RunPage {
        user: &run.submitter,
        game: &run.game,
        category: &run.category,
        verifier: &run.verifier,
        run: &run,
        base,
    };
    state.render("review_run.jinja", ctx)
}

pub async fn verify_run(
    State(state): State<AppState>,
    Path((game_slug, category_id, run_id)): Path<(String, Id<CategoryMarker>, Id<RunMarker>)>,
    user: User,
) -> Result<Redirect, Error> {
    set_verify(&state, game_slug, category_id, run_id, user, 1).await
}

pub async fn reject_run(
    State(state): State<AppState>,
    Path((game_slug, category_id, run_id)): Path<(String, Id<CategoryMarker>, Id<RunMarker>)>,
    user: User,
) -> Result<Redirect, Error> {
    set_verify(&state, game_slug, category_id, run_id, user, -1).await
}

async fn set_verify(
    state: &AppState,
    game_slug: String,
    category_id: Id<CategoryMarker>,
    run_id: Id<RunMarker>,
    user: User,
    value: i16,
) -> Result<Redirect, Error> {
    let (game, member) = game_n_member(state, user, &game_slug).await?;
    if !member.perms.contains(Permissions::VERIFY_RUNS) {
        return Err(Error::InsufficientPermissions);
    }
    let mut trans = state.postgres.begin().await?;
    let run = query!(
        "UPDATE runs SET status = $1, verifier = $2 WHERE id = $3 RETURNING game, category",
        value,
        member.user.id.get(),
        run_id.get()
    )
    .fetch_one(trans.as_mut())
    .await?;
    if Id::new(run.game) != game.id || Id::new(run.category) != category_id {
        trans.rollback().await?;
        return Err(Error::NotFound);
    }
    trans.commit().await?;
    Ok(state.redirect(format!(
        "/game/{game_slug}/category/{category_id}/run/{run_id}"
    )))
}
