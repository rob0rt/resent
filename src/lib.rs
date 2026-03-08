pub mod field;
pub mod mutator;
pub mod predicate;
pub mod privacy;
pub mod query;

pub use resent_macros::EntSchema;

use privacy::EntPrivacyPolicy;
use query::{EntQuery, QueryContext};

pub trait Ent<'ctx, Ctx: 'ctx + Sync>:
    Sized + From<sqlx::postgres::PgRow> + EntPrivacyPolicy<'ctx, Ctx>
{
    const TABLE_NAME: &'static str;

    fn query(context: &'ctx QueryContext<Ctx>) -> EntQuery<'ctx, Ctx, Self> {
        EntQuery::new(context)
    }
}
