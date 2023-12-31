use crate::{
    id::{CategoryMarker, GameMarker, Id},
    AppState, Error,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Encode, Hash, PartialEq, Eq, Clone)]
pub struct Game {
    pub id: Id<GameMarker>,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub default_category: Id<CategoryMarker>,
    pub description: String,
    pub banner: bool,
    pub cover_art: bool,
    pub flags: i64,
}

impl Game {
    pub async fn from_db_slug(state: &AppState, slug: &str) -> Result<Self, Error> {
        match state.get_redis_object(format!("game:{slug}")).await {
            Ok(Some(game)) => return Ok(game),
            Ok(None) => trace!(slug, "did not find game slug in redis cache"),
            Err(source) => error!(
                ?source,
                slug, "an error occured trying to find game slug in redis cache"
            ),
        };
        let Some(game) = query_as!(Game, "SELECT * FROM games WHERE slug = $1", slug)
            .fetch_optional(&state.postgres)
            .await?
        else {
            return Err(Error::NotFound);
        };
        state
            .set_redis_object(format!("game:{slug}"), &game, 600)
            .await?;
        Ok(game)
    }

    pub fn banner_path(&self, ext: &str) -> String {
        format!("/games/{}/banner.{ext}", self.id)
    }

    pub fn cover_art_path(&self, ext: &str) -> String {
        format!("/games/{}/cover_art.{ext}", self.id)
    }

    pub fn banner_url(&self, state: &AppState, ext: &str) -> String {
        if self.banner {
            state.config.user_content_url.clone() + &self.banner_path(ext)
        } else {
            state.static_resource("/defaults/game/banner.svg")
        }
    }

    pub fn cover_art_url(&self, state: &AppState, ext: &str) -> String {
        if self.cover_art {
            state.config.user_content_url.clone() + &self.cover_art_path(ext)
        } else {
            state.static_resource("/defaults/game/coverart.svg")
        }
    }
}
