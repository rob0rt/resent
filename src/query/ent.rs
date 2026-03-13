use crate::{
    Ent, EntEdge,
    field::EntField,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
    query::{
        EntLoadError, EntLoadOnlyError, EntQuery, JoinDef, QueryContext,
        edges::EntWithEdges,
        predicate::{FieldPredicate, QueryPredicate},
        projection::EntFieldProjection,
    },
};
use sea_query::Order;

impl<TEnt: Ent> EntQuery<TEnt> {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            joins: Vec::new(),
            limit: None,
            order: None,
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

    pub fn query_edge<TOtherEnt: Ent>(self) -> EntQuery<TOtherEnt>
    where
        TOtherEnt: EntEdge<TEnt>,
    {
        let filters = if !self.filters.is_empty() || self.limit.is_some() || !self.joins.is_empty()
        {
            vec![
                QueryPredicate::is_in::<TOtherEnt::SourceField, _>(
                    self.select::<TOtherEnt::TargetField>(),
                )
                .to_expr(),
            ]
        } else {
            // Optimization: if the current query has no filters, joins, or limits, we can skip the subquery and just query directly on the edge table
            vec![]
        };
        EntQuery {
            filters: filters,
            joins: Vec::new(),
            limit: None,
            order: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn order_by<TField: EntField<Ent = TEnt>>(mut self, dir: Order) -> Self {
        self.order = Some((TField::NAME.to_string(), dir));
        self
    }

    pub fn select<TField: EntField<Ent = TEnt>>(self) -> EntQuery<EntFieldProjection<TField>> {
        EntQuery {
            filters: self.filters,
            joins: self.joins,
            limit: self.limit,
            order: self.order,
            _marker: std::marker::PhantomData,
        }
    }

    /// A join will include a related entity in the query output
    pub fn join<TOtherEnt: Ent>(self) -> EntQuery<EntWithEdges<TEnt, (TOtherEnt, ())>>
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
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn load<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<Vec<TEnt>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let query_policy = TEnt::query_policy();

        let ctx = ctx;
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
    pub async fn only<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<TEnt, EntLoadOnlyError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let mut results = self.limit(2).load(ctx).await?;
        match results.len() {
            0 => Err(EntLoadOnlyError::NoResults),
            1 => Ok(results.remove(0)),
            _ => Err(EntLoadOnlyError::TooManyResults),
        }
    }

    /// Loads the first result, returning None if there are no results. Will not return an error if there are multiple results.
    pub async fn first<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<Option<TEnt>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let mut results = self.limit(1).load(ctx).await?;
        Ok(results.pop())
    }
}
