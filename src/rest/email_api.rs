//! Implementation of the greeting API. An API that returns a greeting based on a query parameter.

use crate::infra::{
    config::Config,
    error::{ApiError, ApiResult, ClientError},
    extract::Query,
    state::AppState,
};
use axum::{extract::State, routing::post, Router};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, Message, SmtpTransport,
    Transport,
};
use serde::Deserialize;
use std::fmt::Debug;
use tracing::instrument;
use utoipa::IntoParams;

/// Email routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/email", post(send_email))
}

/// Information about the email to send.
#[derive(Debug, Deserialize, IntoParams)]
pub struct EmailParams {
    to: String,
    subject: String,
}

/// A handler for requests to the hello endpoint.
#[utoipa::path(
    post,
    path = "/api/email",
    params(EmailParams),
    responses(
        (status = 201, description = "Success"),
    )
)]
#[instrument(skip(config))]
pub async fn send_email(
    State(config): State<Config>,
    Query(params): Query<EmailParams>,
    body: String,
) -> ApiResult<()> {
    let config = &config.email;

    tracing::info!("Parsing inputs");

    // Parse from and to
    let from = config
        .username
        .parse::<Mailbox>()
        .map_err(|e| ClientError::BadRequest(e.to_string()))?;
    let to = params
        .to
        .parse::<Mailbox>()
        .map_err(|e| ClientError::BadRequest(e.to_string()))?;

    tracing::debug!("Constructing email");

    // Construct email
    let email = Message::builder()
        .from(from.clone())
        .reply_to(from)
        .to(to)
        .subject(&params.subject)
        .body(body)
        .map_err(|e| ClientError::BadRequest(e.to_string()))?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    tracing::debug!("Construct mailer");

    let mailer = SmtpTransport::relay(&config.host)
        .map_err(|e| ApiError::InternalError(e.into()))?
        .credentials(creds)
        .build();

    tracing::info!("Sending mail");

    mailer
        .send(&email)
        .map_err(|e| ApiError::InternalError(e.into()))?;
    Ok(())
}
