use askama::Template;
use askama_axum::IntoResponse;
use axum::{extract::State, response::Redirect, Form, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::infra::{
    database::DbPool,
    error::{ApiResult, ClientError},
    security,
    state::AppState,
};

use super::{index::Index, SESSION_USER_KEY};

pub fn routes() -> Router<AppState> {
    Router::new()
        .typed_get(get_login)
        .typed_post(post_login)
}

#[derive(Template, Default)]
#[template(path = "login.html")]
pub struct LoginTemplate;

#[derive(TypedPath)]
#[typed_path("/login", rejection(ClientError))]
pub struct LoginPath;

/// Display the login page.
#[axum::debug_handler]
pub async fn get_login(_: LoginPath) -> askama_axum::Response {
    LoginTemplate.into_response()
}

#[derive(Serialize, Deserialize)]
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
    let user = security::authenticate(&mut tx, &username, &password).await?;
    session.insert(SESSION_USER_KEY, user).await.unwrap();
    let home = Index.to_string();
    Ok(Redirect::to(&home))
}
