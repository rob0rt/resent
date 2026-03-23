use resent::{
    Ent, EntEdge, EntOptionalEdge, EntSchema,
    context::EntContext,
    privacy::{AlwaysAllowRule, EntMutationPrivacyRule, EntPrivacyPolicy, EntQueryPrivacyRule},
    query::{Order, predicate::QueryPredicate as P},
};
use uuid::Uuid;

struct Context;
impl EntContext for Context {
    fn conn(&self) -> &sqlx::PgPool {
        unimplemented!()
    }
}

#[derive(EntSchema)]
#[entschema(table = "foo")]
#[allow(dead_code)]
pub struct EntFoo {
    #[field(primary_key)]
    id: Uuid,
    name: String,
    bar_id: Uuid,
}

impl EntPrivacyPolicy<Context> for EntFoo {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

#[derive(EntSchema)]
#[entschema(table = "bar")]
#[allow(dead_code)]
pub struct EntBar {
    #[field(primary_key)]
    id: Uuid,
    value: String,
}

impl EntPrivacyPolicy<Context> for EntBar {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

#[derive(EntSchema)]
#[entschema(table = "baz")]
#[allow(dead_code)]
pub struct EntBaz {
    #[field(primary_key)]
    id: Uuid,

    foo_id: Option<Uuid>,
}

impl EntPrivacyPolicy<Context> for EntBaz {
    fn query_policy() -> Vec<Box<dyn EntQueryPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }

    fn mutation_policy() -> Vec<Box<dyn EntMutationPrivacyRule<Self, Context>>> {
        vec![Box::new(AlwaysAllowRule)]
    }
}

impl EntEdge for ent_baz::Id {
    type TargetField = ent_bar::Id;
}

impl EntOptionalEdge for ent_baz::FooId {
    type TargetField = ent_foo::Id;
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
        .query_edge_ref::<ent_baz::Id>()
        .where_field::<ent_baz::Id>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(r#"SELECT * FROM "baz" WHERE "baz"."id" = '{}'"#, uuid),
    );

    let select: sea_query::SelectStatement = EntBar::query()
        .where_field::<ent_bar::Value>(P::equals("hello".to_string()))
        .query_edge_ref::<ent_baz::Id>()
        .where_field::<ent_baz::Id>(P::equals(uuid))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(
            r#"SELECT * FROM "baz" WHERE "baz"."id" IN (SELECT "bar"."id" FROM "bar" WHERE "bar"."value" = 'hello') AND "baz"."id" = '{}'"#,
            uuid
        ),
    );

    let select: sea_query::SelectStatement = EntBaz::query().join::<ent_baz::Id>().into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id""#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .join::<ent_baz::Id>()
        .where_field::<ent_bar::Value, _>(P::equals("hello".to_string()))
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id" WHERE "bar"."value" = 'hello'"#
    );

    let select: sea_query::SelectStatement = EntBaz::query()
        .join::<ent_baz::Id>()
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
        .join::<ent_baz::Id>()
        .where_field::<ent_bar::Value, _>(P::equals("hello".to_string()))
        .where_field::<ent_baz::Id, _>(P::equals(uuid))
        .order_by::<ent_baz::Id, _>(Order::Asc)
        .into();
    assert_eq!(
        select.to_string(sea_query::PostgresQueryBuilder),
        format!(
            r#"SELECT * FROM "baz" INNER JOIN "bar" ON "baz"."id" = "bar"."id" WHERE "bar"."value" = 'hello' AND "baz"."id" = '{}' ORDER BY "baz"."id" ASC"#,
            uuid
        )
    );
}
