//! OpenAPI configuration.

use crate::feature::item::item_repository;
use crate::feature::url::url_repository;
use crate::feature::{
    hello::hello_api, info::info_api, item::item_api, url::url_api, user::user_api,
};
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
        url_api::create_url,
        url_api::visit_url,
        url_api::delete_url,
        url_api::list_urls,
    ),
    components(
        schemas(
            info_api::AppInfo,
            hello_api::Greeting,
            item_repository::NewItem,
            item_repository::Item,
            url_repository::NewShortUrl,
            url_repository::ShortUrl,
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
