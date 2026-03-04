use resent::{
    Ent, EntSchema,
    predicate::QueryPredicate as P,
    privacy::{AlwaysAllowRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntQueryPrivacyRule},
    query::QueryContext,
};
use sea_query::SelectStatement;
use uuid::Uuid;

type Ctx = ();

#[derive(EntSchema)]
#[entschema(table = "foo", ctx = Ctx)]
#[edge(to = EntBar, on = "id", from = "bar_id")]
#[allow(dead_code)]
pub struct EntFoo {
    id: Uuid,
    name: String,
    bar_id: Uuid,
}

impl<'ctx> EntPrivacyPolicy<'ctx, Ctx> for EntFoo {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, Ctx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, Ctx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

#[derive(EntSchema)]
#[entschema(table = "foo", ctx = Ctx)]
#[edge(from = EntFoo, on = "bar_id", to = "id")]
#[allow(dead_code)]
pub struct EntBar {
    id: Uuid,
    value: String,
}

impl<'ctx> EntPrivacyPolicy<'ctx, Ctx> for EntBar {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, Ctx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, Ctx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

#[sqlx::test]
fn test_ent_schema_derive(pool: sqlx::PgPool) {
    let ctx = QueryContext::new(pool, ());
    let (_, f): (&QueryContext<()>, SelectStatement) = EntFoo::query(&ctx)
        .where_name(P::Equals("Test".to_string()))
        .query_bar()
        .into();

    assert_eq!(
        f.to_string(sea_query::PostgresQueryBuilder),
        "SELECT * FROM \"bar\" WHERE \"id\" IN (SELECT \"bar_id\" FROM \"foo\" WHERE \"name\" = 'Test')"
    );
}
