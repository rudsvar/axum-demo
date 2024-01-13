use askama::Template;
use axum::{extract::Query, response::Redirect, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::error::ClientError;

pub fn routes() -> Router {
    Router::new().typed_get(login).typed_get(get_session)
}

const KEY: &str = "name";

#[derive(TypedPath)]
#[typed_path("/login", rejection(ClientError))]
pub struct Login;

#[derive(Deserialize)]
pub struct LoginParams {
    pub user: String,
}

pub async fn login(_: Login, session: Session, Query(params): Query<LoginParams>) -> Redirect {
    session.insert(KEY, params.user).await.unwrap();
    Redirect::to("/session")
}

#[derive(Template)]
#[template(path = "session.html")]
pub struct SessionTemplate {
    user: String,
}

#[derive(TypedPath)]
#[typed_path("/session", rejection(ClientError))]
pub struct GetSession;

pub async fn get_session(_: GetSession, session: Session) -> SessionTemplate {
    let user = match session.get::<String>(KEY).await.unwrap() {
        Some(name) => name,
        None => {
            const DEFAULT: &str = "guest";
            session.insert(KEY, DEFAULT).await.unwrap();
            DEFAULT.to_string()
        }
    };
    SessionTemplate { user }
}
