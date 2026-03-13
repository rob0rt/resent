pub mod field;
pub mod mutator;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use field::EntField;
use privacy::EntPrivacyPolicy;
use query::{EntLoadOnlyError, EntQuery, QueryContext, predicate::QueryPredicate as P};

pub trait Ent: Send + Sized + From<sqlx::postgres::PgRow> {
    const TABLE_NAME: &'static str;

    /// Start an EntQuery for this entity type.
    fn query() -> EntQuery<Self> {
        EntQuery::new()
    }

    /// Load a related entity via an edge.
    fn load_edge<'ctx, TOtherEnt: Ent, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> impl std::future::Future<Output = Result<TOtherEnt, EntLoadOnlyError>> + Send
    where
        Self: EntEdge<TOtherEnt>,
        TOtherEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        self.query_edge().only(context)
    }

    /// Create an EntQuery for an edge, but don't execute it - this is useful for building up more complex queries that involve edges.
    fn query_edge<TOtherEnt: Ent>(&self) -> EntQuery<TOtherEnt>
    where
        Self: EntEdge<TOtherEnt>,
    {
        EntQuery::<TOtherEnt>::new()
            .where_field::<Self::TargetField>(P::equals(Self::SourceField::get_value(self).clone()))
    }

    /// Create an EntQuery for an inbound edge (edge reference)
    fn query_edge_ref<TOtherEnt: Ent>(&self) -> EntQuery<TOtherEnt>
    where
        TOtherEnt: EntEdge<Self>,
    {
        EntQuery::<TOtherEnt>::new().where_field::<TOtherEnt::SourceField>(P::equals(
            TOtherEnt::TargetField::get_value(self).clone(),
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
