use resent::{
    Ent, EntEdgeConfig, EntSchema,
    predicate::QueryPredicate as P,
    privacy::{AlwaysAllowRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntQueryPrivacyRule},
    query::QueryContext,
};
use uuid::Uuid;

type EntCtx = ();

#[derive(EntSchema)]
#[entschema(table = "foo", primary_key = id)]
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
#[entschema(table = "foo", primary_key = id)]
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

#[derive(EntSchema)]
#[entschema(table = "baz", primary_key = id)]
#[allow(dead_code)]
pub struct EntBaz {
    id: Uuid,
}

impl<'ctx> EntPrivacyPolicy<'ctx, EntCtx> for EntBaz {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<'ctx, EntCtx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<'ctx, EntCtx, Self>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

impl EntEdgeConfig<EntBar> for EntBaz {
    type SourceField = ent_baz::Id;
    type TargetField = ent_bar::Id;
}

#[sqlx::test]
fn test_ent_schema_derive(pool: sqlx::PgPool) {
    let ctx = QueryContext::new(pool, ());

    let q = EntBaz::query(&ctx)
        .where_id(P::Equals(Uuid::new_v4()))
        .join::<EntBar>()
        // .where_id(P::Equals(Uuid::new_v4()))
        .foo();

    let p = EntBar::query(&ctx)
        .query::<EntBaz>()
        .where_id(P::Equals(Uuid::new_v4()))
        .load_only()
        .await
        .unwrap();

    let p = p
        .query_edge::<EntBar, _>(&ctx)
        .where_id(P::Equals(Uuid::new_v4()))
        .load_only()
        .await
        .unwrap();

    p.query_edge_ref::<EntBaz, _>(&ctx);

    // let (_, f): (&QueryContext<()>, SelectStatement) = EntFoo::query(&ctx)
    //     .where_name(P::Equals("Test".to_string()))
    //     .query_bar()
    //     .into();

    // let bar = EntBar::load(&ctx, Uuid::new_v4())
    //     .await
    //     .expect("Failed to load EntFoo");

    // let foo = EntFoo::query(&ctx)
    //     .join::<EntBar>()
    //     .filter(ent_bar::fields::Id::predicate(P::InSubquery(f)))
    //     .filter(ent_foo::fields::Name::predicate(P::Equals(
    //         "Test".to_string(),
    //     )))
    //     .foo();

    // let asd = foo.bar_id;
    // let bar: &EntBar = foo.edge();

    // let mut mutator = EntBarMutation {
    //     ent: bar,
    //     id: EntMutationFieldState::Unset,
    //     value: EntMutationFieldState::Unset,
    // };

    // mutator.set::<ent_bar::fields::Value>("New Value".to_string());

    // assert_eq!(
    //     f.to_string(sea_query::PostgresQueryBuilder),
    //     "SELECT * FROM \"bar\" WHERE \"id\" IN (SELECT \"bar_id\" FROM \"foo\" WHERE \"name\" = 'Test')"
    // );
}
