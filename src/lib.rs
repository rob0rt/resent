pub mod field;
pub mod mutator;
pub mod primary_key;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use field::EntField;
use mutator::EntMutator;
use privacy::EntPrivacyPolicy;
use query::{EntLoadOnlyError, EntQuery, QueryContext, predicate::QueryPredicate as P};

use crate::primary_key::EntPrimaryKey;

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

    /// Load an entity by its primary key.
    fn mutate<'a>(&'a self) -> EntMutator<'a, Self> {
        EntMutator::new(self)
    }

    /// Load a related entity via an edge.
    fn load_edge<'ctx, TOtherEnt, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> impl std::future::Future<Output = Result<TOtherEnt, EntLoadOnlyError>> + Send
    where
        Self: EntEdge<TOtherEnt>,
        TOtherEnt: Ent + EntPrivacyPolicy<'ctx, Ctx>,
    {
        self.query_edge().only(context)
    }

    /// Create an EntQuery for an edge, but don't execute it - this is useful for building up more complex queries that involve edges.
    fn query_edge<TOtherEnt: Ent>(&self) -> EntQuery<TOtherEnt>
    where
        Self: EntEdge<TOtherEnt>,
    {
        EntQuery::<TOtherEnt>::new().where_field::<Self::TargetField>(P::equals(
            <Self::SourceField as EntField>::get_value(self).clone(),
        ))
    }

    /// Create an EntQuery for an inbound edge (edge reference)
    fn query_edge_ref<TOtherEnt>(&self) -> EntQuery<TOtherEnt>
    where
        TOtherEnt: Ent + EntEdge<Self>,
    {
        EntQuery::<TOtherEnt>::new().where_field::<TOtherEnt::SourceField>(P::equals(
            <TOtherEnt::TargetField as EntField>::get_value(self).clone(),
        ))
    }
}

pub trait EntEdge<TTarget: Ent>
where
    Self: Ent,
{
    type SourceField: EntField<Ent = Self>;
    type TargetField: EntField<Ent = TTarget, Value = <Self::SourceField as EntField>::Value>;
}
