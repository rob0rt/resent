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

    fn query<'ctx, Ctx: 'ctx + Sync>(context: &'ctx QueryContext<Ctx>) -> EntQuery<'ctx, Ctx, Self>
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        EntQuery::new(context)
    }
}

pub trait EntEdgeConfig<TTarget: Ent>
where
    Self: Ent,
{
    type SourceField: EntField<Self>;
    type TargetField: EntField<TTarget>;
}
