mod edges;
mod ent;
pub mod predicate;
mod projection;

use sea_query::{Expr, ExprTrait, Order};

use crate::{
    Ent,
    field::EntField,
    query::{edges::EntWithEdges, projection::EntFieldProjection},
};

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

impl<'ctx, Ctx: 'ctx + Sync, TOut> EntQuery<'ctx, Ctx, TOut> {
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
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

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent, Target: EntTarget<Target = TEnt>>
    Into<sea_query::SelectStatement> for EntQuery<'ctx, Ctx, Target>
{
    fn into(self) -> sea_query::SelectStatement {
        let mut query = sea_query::Query::select();
        query
            .column(sea_query::Asterisk)
            .from(sea_query::Alias::new(TEnt::TABLE_NAME));
        for join in self.joins {
            query.join(
                sea_query::JoinType::InnerJoin,
                sea_query::Alias::new(join.table),
                Expr::col((join.left_table, join.left_col))
                    .equals((join.right_table, join.right_col)),
            );
        }
        for expr in self.filters {
            query.and_where(expr);
        }
        if let Some(limit) = self.limit {
            query.limit(limit as u64);
        }
        if let Some(order) = self.order {
            query.order_by((TEnt::TABLE_NAME, order.0), order.1.into());
        }
        query
    }
}
