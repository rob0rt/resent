pub mod cache;
pub mod context;
pub mod creator;
pub mod field;
pub mod mutator;
pub mod primary_key;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use crate::{
    context::EntContext,
    creator::EntCreator,
    field::EntField,
    mutator::EntMutator,
    primary_key::EntPrimaryKey,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
    query::{EntLoadOnlyError, EntQuery, predicate::QueryPredicate as P},
};
use sea_query::DeleteStatement;
use sea_query_sqlx::SqlxBinder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntDeletionError {
    #[error("Database error: {0}")]
    QueryError(#[from] sqlx::Error),
}

pub trait Ent:
    Send + Sync + Sized + Clone + 'static + for<'a> From<&'a sqlx::postgres::PgRow>
{
    const TABLE_NAME: &'static str;
    type PrimaryKey: EntPrimaryKey<Self>;

    /// Start an EntQuery for this entity type.
    fn query() -> EntQuery<Self> {
        EntQuery::new()
    }

    fn create() -> EntCreator<Self> {
        EntCreator::new()
    }

    fn load<TCtx: EntContext>(
        ctx: &TCtx,
        primary_key: <Self::PrimaryKey as EntPrimaryKey<Self>>::Value,
    ) -> impl std::future::Future<Output = Result<Self, EntLoadOnlyError>> + Send
    where
        Self: EntPrivacyPolicy<TCtx> + 'static,
    {
        async {
            // Check cache first
            if let Some(cached) = ctx.cache().get::<Self>(&primary_key).await {
                let policies = Self::query_policy();
                for policy in &policies {
                    match policy.evaluation(ctx, &cached).await {
                        PrivacyRuleOutcome::Allow => return Ok(cached),
                        PrivacyRuleOutcome::Deny => return Err(EntLoadOnlyError::NoResults),
                        PrivacyRuleOutcome::Skip => continue,
                    }
                }
                return Err(EntLoadOnlyError::NoResults);
            }

            // Cache miss
            Self::query()
                .where_expr(Self::PrimaryKey::as_expr(primary_key))
                .only(ctx)
                .await
        }
    }

    /// Delete this entity from the database.
    fn delete<TCtx: EntContext>(
        self,
        ctx: &TCtx,
    ) -> impl std::future::Future<Output = Result<(), EntDeletionError>> + Send
    where
        Self: EntPrivacyPolicy<TCtx> + 'static,
    {
        async move {
            let primary_key = Self::PrimaryKey::get_value(&self);

            let (sql, values) = DeleteStatement::new()
                .from_table(Self::TABLE_NAME)
                .cond_where(Self::PrimaryKey::as_expr(primary_key.clone()))
                .build_sqlx(sea_query::PostgresQueryBuilder);

            sqlx::query_with(&sql, values).execute(ctx.conn()).await?;

            ctx.cache().invalidate::<Self>(&primary_key).await;

            Ok(())
        }
    }

    /// Load an entity by its primary key.
    fn mutate<'a>(&'a self) -> EntMutator<'a, Self> {
        EntMutator::new(self)
    }

    /// Load a related entity via an edge, using the target entity's cache.
    fn load_edge<TEdge: EntEdge<Ent = Self>, TCtx: EntContext>(
        &self,
        ctx: &TCtx,
    ) -> impl std::future::Future<
        Output = Result<<TEdge::TargetField as EntField>::Ent, EntLoadOnlyError>,
    > + Send
    where
        TEdge::Value: std::hash::Hash + Eq,
        <TEdge::TargetField as EntField>::Ent:
            EntPrivacyPolicy<TCtx> + Ent<PrimaryKey = TEdge::TargetField>,
    {
        <TEdge::TargetField as EntField>::Ent::load(ctx, TEdge::get_value(self).clone())
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

    /// Load a related entity via an optional edge. Returns `Ok(None)` if the edge field is `None`.
    fn load_optional_edge<TEdge: EntOptionalEdge<Ent = Self>, TCtx: EntContext>(
        &self,
        ctx: &TCtx,
    ) -> impl std::future::Future<
        Output = Result<Option<<TEdge::TargetField as EntField>::Ent>, EntLoadOnlyError>,
    > + Send
    where
        <TEdge::TargetField as EntField>::Ent: EntPrivacyPolicy<TCtx>,
    {
        let query = self.query_optional_edge::<TEdge>();
        async move {
            match query {
                None => Ok(None),
                Some(q) => q.only(ctx).await.map(Some),
            }
        }
    }

    /// Create an `EntQuery` for an optional edge. Returns `None` if the edge field value is `None`.
    fn query_optional_edge<TEdge: EntOptionalEdge<Ent = Self>>(
        &self,
    ) -> Option<EntQuery<<TEdge::TargetField as EntField>::Ent>> {
        TEdge::get_value(self).as_ref().map(|v| {
            EntQuery::<<TEdge::TargetField as EntField>::Ent>::new()
                .where_field::<TEdge::TargetField>(P::equals(v.clone()))
        })
    }

    /// Create an `EntQuery` for an inbound optional edge reference (finds entities whose optional
    /// FK field references `self`).
    fn query_optional_edge_ref<TEdge: EntOptionalEdge>(&self) -> EntQuery<TEdge::Ent>
    where
        TEdge::TargetField: EntField<Ent = Self>,
    {
        EntQuery::<TEdge::Ent>::new().where_field::<TEdge>(P::equals(Some(
            <TEdge::TargetField as EntField>::get_value(self).clone(),
        )))
    }
}

pub trait EntEdge
where
    Self: EntField,
{
    type TargetField: EntField<Value = Self::Value>;
}

pub trait EntOptionalEdge
where
    Self: EntField<Value = Option<<Self::TargetField as EntField>::Value>>,
{
    type TargetField: EntField;
}
