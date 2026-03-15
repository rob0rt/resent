mod edges;
mod ent;
pub mod predicate;
mod projection;

use crate::{
    Ent,
    field::EntField,
    query::{edges::EntWithEdges, projection::EntFieldProjection},
};
pub use sea_query::Order;
use sea_query::{Expr, ExprTrait};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntLoadError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Invalid privacy policy")]
    InvalidPrivacyPolicy,
}

#[derive(Debug, Error)]
pub enum EntLoadOnlyError {
    #[error("Load error: {0}")]
    LoadError(#[from] EntLoadError),
    #[error("No results found")]
    NoResults,
    #[error("Too many results found")]
    TooManyResults,
}

#[derive(Clone)]
pub struct QueryContext<T> {
    pub(crate) conn: sqlx::PgPool,
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

pub struct EntQuery<TOut> {
    filters: Vec<Expr>,
    joins: Vec<JoinDef>,
    limit: Option<usize>,
    order: Option<(String, Order)>,
    _marker: std::marker::PhantomData<TOut>,
}

impl<TOut> EntQuery<TOut> {
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a raw filter expression to the query - this is used internally for
    /// things like filtering by primary key, but can also be used for more
    /// complex queries that aren't directly supported by the other query
    /// methods.
    pub(crate) fn where_expr(mut self, expr: Expr) -> Self {
        self.filters.push(expr);
        self
    }
}

trait EntTarget {
    type Target: Ent;
}
impl<TEnt: Ent> EntTarget for TEnt {
    type Target = TEnt;
}
impl<TEnt: Ent, TEdges> EntTarget for EntWithEdges<TEnt, TEdges> {
    type Target = TEnt;
}
impl<TEnt: Ent, TEdges> EntTarget for (TEnt, TEdges) {
    type Target = TEnt;
}
impl<TEnt: Ent, TField: EntField<Ent = TEnt>> EntTarget for EntFieldProjection<TField> {
    type Target = TEnt;
}

impl<TEnt: Ent, Target: EntTarget<Target = TEnt>> From<EntQuery<Target>>
    for sea_query::SelectStatement
{
    fn from(val: EntQuery<Target>) -> Self {
        let mut query = sea_query::Query::select();
        query
            .column(sea_query::Asterisk)
            .from(sea_query::Alias::new(TEnt::TABLE_NAME));
        for join in val.joins {
            query.join(
                sea_query::JoinType::InnerJoin,
                sea_query::Alias::new(join.table),
                Expr::col((join.left_table, join.left_col))
                    .equals((join.right_table, join.right_col)),
            );
        }
        for expr in val.filters {
            query.and_where(expr);
        }
        if let Some(limit) = val.limit {
            query.limit(limit as u64);
        }
        if let Some(order) = val.order {
            query.order_by((TEnt::TABLE_NAME, order.0), order.1);
        }
        query
    }
}
