pub mod field;
pub mod mutator;
pub mod primary_key;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use field::EntField;
use mutator::EntMutator;
use primary_key::EntPrimaryKey;
use privacy::EntPrivacyPolicy;
use query::{EntLoadOnlyError, EntQuery, QueryContext, predicate::QueryPredicate as P};
use sea_query::DeleteStatement;
use sea_query_sqlx::SqlxBinder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntDeletionError {
    #[error("Database error: {0}")]
    QueryError(#[from] sqlx::Error),
}

pub trait Ent: Send + Sized + for<'a> From<&'a sqlx::postgres::PgRow> {
    const TABLE_NAME: &'static str;
    type PrimaryKey: EntPrimaryKey<Self>;

    /// Start an EntQuery for this entity type.
    fn query() -> EntQuery<Self> {
        EntQuery::new()
    }

    fn load<'ctx, Ctx: 'ctx + Sync>(
        context: &'ctx QueryContext<Ctx>,
        primary_key: <Self::PrimaryKey as EntPrimaryKey<Self>>::Value,
    ) -> impl std::future::Future<Output = Result<Self, EntLoadOnlyError>> + Send
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        async {
            Self::query()
                .where_expr(Self::PrimaryKey::as_expr(primary_key))
                .only(context)
                .await
        }
    }

    /// Delete this entity from the database.
    fn delete<'ctx, Ctx: 'ctx + Sync>(
        self,
        context: &'ctx QueryContext<Ctx>,
    ) -> impl std::future::Future<Output = Result<(), EntDeletionError>> + Send
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        async move {
            let primary_key = Self::PrimaryKey::get_value(&self);

            let (sql, values) = DeleteStatement::new()
                .from_table(Self::TABLE_NAME)
                .cond_where(Self::PrimaryKey::as_expr(primary_key))
                .build_sqlx(sea_query::PostgresQueryBuilder);

            sqlx::query_with(&sql, values)
                .execute(&context.conn)
                .await?;

            Ok(())
        }
    }

    /// Load an entity by its primary key.
    fn mutate<'a>(&'a self) -> EntMutator<'a, Self> {
        EntMutator::new(self)
    }

    /// Load a related entity via an edge.
    fn load_edge<'ctx, TEdge: EntEdge<Ent = Self>, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> impl std::future::Future<
        Output = Result<<TEdge::TargetField as EntField>::Ent, EntLoadOnlyError>,
    > + Send
    where
        <TEdge::TargetField as EntField>::Ent: EntPrivacyPolicy<'ctx, Ctx>,
    {
        self.query_edge::<TEdge>().only(context)
    }

    /// Create an EntQuery for an edge, but don't execute it - this is useful for building up more complex queries that
    /// involve edges.
    fn query_edge<TEdge: EntEdge<Ent = Self>>(
        &self,
    ) -> EntQuery<<TEdge::TargetField as EntField>::Ent> {
        EntQuery::<<TEdge::TargetField as EntField>::Ent>::new()
            .where_field::<TEdge::TargetField>(P::equals(TEdge::get_value(self).clone()))
    }

    /// Create an EntQuery for an inbound edge (edge reference)
    fn query_edge_ref<TEdge: EntEdge>(&self) -> EntQuery<TEdge::Ent>
    where
        TEdge::TargetField: EntField<Ent = Self>,
    {
        EntQuery::<TEdge::Ent>::new().where_field::<TEdge>(P::equals(
            <TEdge::TargetField as EntField>::get_value(self).clone(),
        ))
    }
}

pub trait EntEdge
where
    Self: EntField,
{
    type TargetField: EntField<Value = Self::Value>;
}
