use crate::infra::error::ServiceResult;
use axum::async_trait;
use sqlx::{Postgres, Transaction};

pub mod item_repository;
pub mod user_repository;

pub type Tx = Transaction<'static, Postgres>;

#[async_trait]
pub trait Repository {
    type NewEntity;
    type Entity;

    async fn create(&self, tx: &mut Tx, e: Self::NewEntity) -> ServiceResult<Self::Entity>;
    async fn update(&self, tx: &mut Tx, id: i32, e: Self::NewEntity)
        -> ServiceResult<Self::Entity>;
    async fn read(&self, tx: &mut Tx, id: i32) -> ServiceResult<Self::Entity>;
    async fn delete(&self, tx: &mut Tx, id: i32) -> ServiceResult<Self::Entity>;
    async fn list(&self, tx: &mut Tx) -> ServiceResult<Self::Entity>;
}
