use resent::{
    Ent, EntSchema,
    field::{EntField, EntFieldGetter, EntFieldSetter},
    mutator::{EntMutationField, EntMutationFieldState, EntMutator},
    predicate::{EntQueryPredicate, QueryPredicate as P},
    privacy::{AlwaysAllowRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntQueryPrivacyRule},
    query::QueryContext,
};
use sea_query::SelectStatement;
use uuid::Uuid;

type EntCtx = ();

#[derive(EntSchema)]
#[entschema(table = "foo", ctx = EntCtx, primary_key = id)]
#[edge(to = EntBar, on = "id", from = "bar_id")]
#[allow(dead_code)]
pub struct EntFoo {
    id: Uuid,
    name: String,
    bar_id: Uuid,
}

impl<'ctx> EntPrivacyPolicy<'ctx, EntCtx> for EntFoo {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, EntCtx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, EntCtx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

#[derive(EntSchema)]
#[entschema(table = "foo", ctx = EntCtx, primary_key = id)]
#[edge(from = EntFoo, on = "bar_id", to = "id")]
#[allow(dead_code)]
pub struct EntBar {
    id: Uuid,
    value: String,
}

impl<'ctx> EntPrivacyPolicy<'ctx, EntCtx> for EntBar {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, EntCtx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, EntCtx, Self>>> {
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

    EntFoo::query(&ctx).filter(EntQueryPredicate::<_, _, _, ent_foo::fields::Name>::equals(
        "Test".to_string(),
    ));

    let bar = EntBar::load(&ctx, Uuid::new_v4())
        .await
        .expect("Failed to load EntFoo");

    let mut mutator = EntBarMutation {
        ent: bar,
        id: EntMutationFieldState::Unset,
        value: EntMutationFieldState::Unset,
    };

    mutator.set::<ent_bar::fields::Value>("New Value".to_string());

    assert_eq!(
        f.to_string(sea_query::PostgresQueryBuilder),
        "SELECT * FROM \"bar\" WHERE \"id\" IN (SELECT \"bar_id\" FROM \"foo\" WHERE \"name\" = 'Test')"
    );
}
