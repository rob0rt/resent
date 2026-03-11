pub mod field;
pub mod mutator;
pub mod predicate;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use privacy::EntPrivacyPolicy;
use query::{EntQuery, QueryContext};

use crate::field::EntField;

pub trait Ent: Sized + From<sqlx::postgres::PgRow> {
    const TABLE_NAME: &'static str;

    /// Start an EntQuery for this entity type.
    fn query<'ctx, Ctx: 'ctx + Sync>(context: &'ctx QueryContext<Ctx>) -> EntQuery<'ctx, Ctx, Self>
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        EntQuery::new(context)
    }

    /// Load a related entity via an edge.
    fn load_edge<'ctx, TOtherEnt: Ent, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> TOtherEnt
    where
        Self: EntEdgeConfig<TOtherEnt>,
        TOtherEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let query = EntQuery::<_, TOtherEnt>::new(context);
        query.
        unimplemented!()
    }

    fn query_edge<'ctx, TOtherEnt: Ent, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> EntQuery<'ctx, Ctx, TOtherEnt>
    where
        Self: EntEdgeConfig<TOtherEnt>,
        TOtherEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        unimplemented!()
    }

    fn query_edge_ref<'ctx, TOtherEnt: Ent, Ctx: 'ctx + Sync>(
        &self,
        context: &'ctx QueryContext<Ctx>,
    ) -> EntQuery<'ctx, Ctx, TOtherEnt>
    where
        TOtherEnt: EntEdgeConfig<Self>,
        TOtherEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        unimplemented!()
    }
}

pub trait EntEdgeConfig<TTarget: Ent>
where
    Self: Ent,
{
    type SourceField: EntField<Ent = Self>;
    type TargetField: EntField<Ent = TTarget, Value = <Self::SourceField as EntField>::Value>;
}
