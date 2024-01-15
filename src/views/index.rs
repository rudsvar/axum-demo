use askama::Template;
use axum::{extract::State, response::Redirect, Form, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::{
    database::DbPool,
    error::{ApiResult, ClientError},
    security::{self, User},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .typed_post(login)
        .typed_get(index)
        .typed_get(logout)
}

const SESSION_USER_KEY: &str = "user";

#[derive(Template, Default)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    error: String,
}

impl LoginTemplate {
    pub fn with_error(error: String) -> Self {
        Self { error }
    }
}

#[derive(TypedPath)]
#[typed_path("/login", rejection(ClientError))]
pub struct Login;

#[derive(Deserialize)]
pub struct LoginParams {
    pub username: String,
    pub password: String,
}

pub async fn login(
    _: Login,
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
    Ok(Redirect::to("/"))
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    username: String,
}

#[derive(TypedPath)]
#[typed_path("/", rejection(ClientError))]
pub struct Index;

pub async fn index(_: Index, user: User) -> IndexTemplate {
    // Display user information
    IndexTemplate {
        username: user.username().to_string(),
    }
}

#[derive(TypedPath)]
#[typed_path("/logout", rejection(ClientError))]
pub struct Logout;

pub async fn logout(_: Logout, session: Session) -> Redirect {
    session.delete().await.unwrap();
    Redirect::to("/")
}
