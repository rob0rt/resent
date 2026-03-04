pub mod field;
pub mod predicate;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use privacy::EntPrivacyPolicy;
use query::{EntQuery, QueryContext};

pub trait Ent<'ctx, Ctx: 'ctx + Sync = ()>: Sized + From<sqlx::postgres::PgRow> {
    const TABLE_NAME: &'static str;

    fn query(context: &'ctx QueryContext<Ctx>) -> EntQuery<'ctx, Ctx, Self>
    where
        Self: EntPrivacyPolicy<'ctx, Ctx>,
    {
        EntQuery::new(context)
    }
}
