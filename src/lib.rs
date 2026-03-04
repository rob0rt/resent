mod predicate;
mod privacy;
mod query;

pub use predicate::{EntField, EntFieldPredicate, QueryPredicate};
pub use privacy::{
    AlwaysAllowRule, AlwaysDenyRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntPrivacyRule,
    EntQueryPrivacyRule, PrivacyRuleOutcome,
};
pub use query::{EntQuery, QueryContext};
pub use resent_macros::EntSchema;

pub trait Ent<'ctx, Ctx: 'ctx + Sync = ()>: Sized + From<sqlx::postgres::PgRow> {
    const TABLE_NAME: &'static str;

    fn query(context: &'ctx QueryContext<Ctx>) -> EntQuery<'ctx, Ctx, Self>
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        EntQuery::new(context)
    }
}
