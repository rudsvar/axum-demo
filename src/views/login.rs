use askama::Template;
use axum::{extract::State, response::Redirect, Form, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::{
    database::DbPool,
    error::{ApiResult, ClientError},
    security,
    state::AppState,
};

use super::{index::Index, SESSION_USER_KEY};

pub fn routes() -> Router<AppState> {
    Router::new().typed_get(get_login).typed_post(post_login)
}

#[derive(Template, Default)]
#[template(path = "login.html")]
pub struct LoginTemplate;

#[derive(TypedPath)]
#[typed_path("/login", rejection(ClientError))]
pub struct LoginPath;

/// Display the login page.
pub async fn get_login(_: LoginPath) -> LoginTemplate {
    LoginTemplate
}

#[derive(Deserialize)]
pub struct LoginParams {
    pub username: String,
    pub password: String,
}

pub async fn post_login(
    _: LoginPath,
    session: Session,
    db: State<DbPool>,
    Form(params): Form<LoginParams>,
) -> ApiResult<Redirect> {
    let mut tx = db.begin().await.unwrap();
    let username = params.username;
    let password = params.password;
    let user = match security::authenticate(&mut tx, &username, &password).await {
        Ok(user) => user,
        Err(e) => return Ok(Redirect::to(&format!("/?error={}", e))),
    };
    session.insert(SESSION_USER_KEY, user).await.unwrap();
    let home = Index.to_string();
    Ok(Redirect::to(&home))
}
