pub trait EntContext: Send + Sync {
    fn conn(&self) -> &sqlx::PgPool;
}
