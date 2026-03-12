mod edges;
mod ent;
pub mod predicate;
mod projection;

use sea_query::{Expr, Order};

#[derive(Debug)]
pub enum EntLoadError {
    DatabaseError(sqlx::Error),
    InvalidPrivacyPolicy,
}

#[derive(Debug)]
pub enum EntLoadOnlyError {
    LoadError(EntLoadError),
    NoResults,
    TooManyResults,
}

impl From<EntLoadError> for EntLoadOnlyError {
    fn from(value: EntLoadError) -> Self {
        EntLoadOnlyError::LoadError(value)
    }
}

#[derive(Clone)]
pub struct QueryContext<T> {
    conn: sqlx::PgPool,
    pub data: T,
}

impl<T> QueryContext<T> {
    pub fn new(conn: sqlx::PgPool, data: T) -> Self {
        Self { conn, data }
    }

    pub fn with<R>(&self, data: R) -> QueryContext<R> {
        QueryContext {
            conn: self.conn.clone(),
            data,
        }
    }
}

struct JoinDef {
    table: &'static str,
    left_table: &'static str,
    left_col: &'static str,
    right_table: &'static str,
    right_col: &'static str,
}

pub struct EntQuery<'ctx, Ctx: 'ctx + Sync, TOut> {
    filters: Vec<Expr>,
    joins: Vec<JoinDef>,
    limit: Option<usize>,
    order: Option<(String, Order)>,
    ctx: &'ctx QueryContext<Ctx>,
    _marker: std::marker::PhantomData<TOut>,
}
