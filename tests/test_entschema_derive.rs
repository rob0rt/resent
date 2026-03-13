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
#[entschema(table = "bar")]
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

// #[sqlx::test]
#[tokio::test]
async fn test_ent_schema_derive() {
    let uuid = Uuid::new_v4();

    let select: sea_query::SelectStatement = EntBaz::query().into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz""#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .order_by::<ent_baz::Id>(sea_query::Order::Desc)
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" ORDER BY "baz"."id" DESC"#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .where_field::<ent_baz::Id>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(r#"SELECT * FROM "baz" WHERE "baz"."id" = '{}'"#, uuid),
    );

    let select: sea_query::SelectStatement = EntBaz::query().limit(2).into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" LIMIT 2"#
    );

    let select: sea_query::SelectStatement = EntBar::query()
        .query_edge::<EntBaz>()
        .where_field::<ent_baz::Id>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(r#"SELECT * FROM "baz" WHERE "baz"."id" = '{}'"#, uuid),
    );

    let select: sea_query::SelectStatement = EntBar::query()
        .where_field::<ent_bar::Value>(P::equals("hello".to_string()))
        .query_edge::<EntBaz>()
        .where_field::<ent_baz::Id>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(
            r#"SELECT * FROM "baz" WHERE "baz"."id" IN (SELECT "bar"."id" FROM "bar" WHERE "bar"."value" = 'hello') AND "baz"."id" = '{}'"#,
            uuid
        ),
    );

    let select: sea_query::SelectStatement = EntBaz::query().join::<EntBar>().into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id""#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .join::<EntBar>()
        .where_field::<ent_bar::Value, _>(P::equals("hello".to_string()))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id" WHERE "bar"."value" = 'hello'"#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .join::<EntBar>()
        .where_field::<ent_bar::Value, _>(P::equals("hello".to_string()))
        .where_field::<ent_baz::Id, _>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(
            r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id" WHERE "bar"."value" = 'hello' AND "baz"."id" = '{}'"#,
            uuid
        )
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .join::<EntBar>()
        .where_field::<ent_bar::Value, _>(P::equals("hello".to_string()))
        .where_field::<ent_baz::Id, _>(P::equals(uuid))
        .order_by::<ent_baz::Id, _>(sea_query::Order::Asc)
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(
            r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id" WHERE "bar"."value" = 'hello' AND "baz"."id" = '{}' ORDER BY "baz"."id" ASC"#,
            uuid
        )
    );
}
