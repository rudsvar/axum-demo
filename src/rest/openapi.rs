//! OpenAPI configuration.

use super::{hello_api, info_api, integration_api, item_api, user_api};
use crate::core::item::item_repository;
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

/// OpenApi configuration.
#[derive(OpenApi)]
#[openapi(
    paths(
        info_api::info,
        hello_api::hello,
        item_api::create_item,
        item_api::list_items,
        item_api::update_item,
        item_api::delete_item,
        item_api::stream_items,
        user_api::user,
        user_api::admin,
        integration_api::remote_items,
    ),
    components(
        schemas(
            info_api::AppInfo,
            hello_api::Greeting,
            item_repository::NewItem,
            item_repository::Item,
            crate::infra::error::ErrorBody
        )
    ),
    modifiers(&SecurityAddon)
)]
#[derive(Clone, Copy, Debug)]
pub struct ApiDoc;

/// Security settings
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "basic",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)),
            )
        }
    }
}
