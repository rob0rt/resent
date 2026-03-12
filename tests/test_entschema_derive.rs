use resent::{
    Ent, EntEdge, EntSchema,
    privacy::{AlwaysAllowRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntQueryPrivacyRule},
    query::{QueryContext, predicate::QueryPredicate as P},
};
use uuid::Uuid;

type EntCtx = ();

#[derive(EntSchema)]
#[entschema(table = "foo")]
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
#[entschema(table = "foo")]
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
#[entschema(table = "baz")]
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

impl EntEdge<EntBar> for EntBaz {
    type SourceField = ent_baz::Id;
    type TargetField = ent_bar::Id;
}

#[sqlx::test]
fn test_ent_schema_derive(pool: sqlx::PgPool) {
    let ctx = QueryContext::new(pool, ());

    let q = EntBaz::query(&ctx)
        .join::<EntBar>()
        .where_field::<ent_bar::Id, _>(P::equals(Uuid::new_v4()));

    let p = EntBar::query(&ctx)
        .query_edge::<EntBaz>()
        .where_field::<ent_baz::Id>(P::equals(Uuid::new_v4()))
        .load_only()
        .await
        .unwrap();

    let p = p
        .query_edge::<EntBar, _>(&ctx)
        .where_field::<ent_bar::Id>(P::equals(Uuid::new_v4()))
        .load_only()
        .await
        .unwrap();

    p.query_edge_ref::<EntBaz, _>(&ctx);
}
