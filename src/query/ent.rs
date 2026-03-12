use crate::{
    Ent, EntEdge,
    field::EntField,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
    query::{
        EntLoadError, EntLoadOnlyError, EntQuery, JoinDef, QueryContext, edges::EntWithEdges,
        predicate::FieldPredicate, projection::EntFieldProjection,
    },
};
use sea_query::{Expr, ExprTrait, Order};

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent> EntQuery<'ctx, Ctx, TEnt> {
    pub fn new(ctx: &'ctx QueryContext<Ctx>) -> Self {
        Self {
            filters: Vec::new(),
            joins: Vec::new(),
            limit: None,
            order: None,
            ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn where_field<TField: EntField<Ent = TEnt>>(
        mut self,
        field_query: impl FieldPredicate<TField>,
    ) -> Self {
        self.filters.push(field_query.to_expr());
        self
    }

    pub fn query_edge<TOtherEnt: Ent>(self) -> EntQuery<'ctx, Ctx, TOtherEnt>
    where
        TOtherEnt: EntEdge<TEnt>,
    {
        let ctx = self.ctx;
        let mut subquery = sea_query::Query::select();
        subquery
            .column(sea_query::Alias::new(
                <TOtherEnt as EntEdge<TEnt>>::TargetField::NAME,
            ))
            .from(sea_query::Alias::new(TEnt::TABLE_NAME));
        for expr in self.filters {
            subquery.and_where(expr);
        }
        if let Some(limit) = self.limit {
            subquery.limit(limit as u64);
        }
        let filter = Expr::col((
            TOtherEnt::TABLE_NAME,
            <TOtherEnt as EntEdge<TEnt>>::SourceField::NAME,
        ))
        .in_subquery(subquery.to_owned());
        EntQuery {
            filters: vec![filter],
            joins: Vec::new(),
            limit: None,
            order: None,
            ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn order_by<TField: EntField<Ent = TEnt>>(mut self, dir: Order) -> Self {
        self.order = Some((TField::NAME.to_string(), dir));
        self
    }

    pub fn select<TField: EntField<Ent = TEnt>>(
        self,
    ) -> EntQuery<'ctx, Ctx, EntFieldProjection<TField>> {
        EntQuery {
            filters: self.filters,
            joins: self.joins,
            limit: self.limit,
            order: self.order,
            ctx: self.ctx,
            _marker: std::marker::PhantomData,
        }
    }

    /// A join will include a related entity in the query output
    pub fn join<TOtherEnt: Ent>(self) -> EntQuery<'ctx, Ctx, EntWithEdges<TEnt, (TOtherEnt, ())>>
    where
        TEnt: EntEdge<TOtherEnt>,
    {
        let mut joins = self.joins;
        joins.push(JoinDef {
            table: TOtherEnt::TABLE_NAME,
            left_table: TEnt::TABLE_NAME,
            left_col: <TEnt as EntEdge<TOtherEnt>>::SourceField::NAME,
            right_table: TOtherEnt::TABLE_NAME,
            right_col: <TEnt as EntEdge<TOtherEnt>>::TargetField::NAME,
        });
        EntQuery {
            filters: self.filters,
            joins,
            limit: self.limit,
            order: self.order,
            ctx: self.ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn load(self) -> Result<Vec<TEnt>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let query_policy = TEnt::query_policy();

        let ctx = self.ctx;
        let select: sea_query::SelectStatement = self.into();
        let select_statement = select.to_string(sea_query::PostgresQueryBuilder);

        let conn = &ctx.conn;
        let ents = sqlx::query(&select_statement)
            .fetch_all(conn)
            .await
            .map_err(EntLoadError::DatabaseError)?
            .into_iter()
            .map(|row| TEnt::from(row));

        let mut results = Vec::new();
        'ents: for ent in ents {
            'rules: for rule in &query_policy {
                match rule.evaluation(ctx, &ent).await {
                    PrivacyRuleOutcome::Allow => {
                        results.push(ent);
                        continue 'ents;
                    }
                    PrivacyRuleOutcome::Deny => {
                        continue 'ents;
                    }
                    PrivacyRuleOutcome::Skip => continue 'rules,
                }
            }

            println!(
                "Warning: No privacy rules applied for query on {}",
                std::any::type_name::<TEnt>()
            );
        }

        Ok(results)
    }

    /// Loads a single entity, returning an error if there are zero or more than one results.
    pub async fn load_only(self) -> Result<TEnt, EntLoadOnlyError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let mut results = self.limit(2).load().await?;
        match results.len() {
            0 => Err(EntLoadOnlyError::NoResults),
            1 => Ok(results.remove(0)),
            _ => Err(EntLoadOnlyError::TooManyResults),
        }
    }
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent> Into<sea_query::SelectStatement>
    for EntQuery<'ctx, Ctx, TEnt>
{
    fn into(self) -> sea_query::SelectStatement {
        let mut query = sea_query::Query::select();
        query
            .column(sea_query::Asterisk)
            .from(sea_query::Alias::new(TEnt::TABLE_NAME));
        for join in self.joins {
            query.join(
                sea_query::JoinType::InnerJoin,
                sea_query::Alias::new(join.table),
                Expr::col((join.left_table, join.left_col))
                    .equals((join.right_table, join.right_col)),
            );
        }
        for expr in self.filters {
            query.and_where(expr);
        }
        if let Some(limit) = self.limit {
            query.limit(limit as u64);
        }
        if let Some(order) = self.order {
            query.order_by((TEnt::TABLE_NAME, order.0), order.1.into());
        }
        query
    }
}
