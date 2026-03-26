use crate::cache::EntCache;

pub trait EntContext: Send + Sync {
    fn conn(&self) -> &sqlx::PgPool;
    fn cache(&self) -> &EntCache;
}
