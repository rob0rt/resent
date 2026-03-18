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
use futures_util::StreamExt;
use sea_query::Order;
use sea_query_sqlx::SqlxBinder;

impl<TEnt: Ent> EntQuery<TEnt> {
    pub(crate) fn new() -> Self {
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

    /// Query an outbound edge (regular edge)
    pub fn query_edge<TEdge: EntEdge<Ent = TEnt>>(
        self,
    ) -> EntQuery<<TEdge::TargetField as EntField>::Ent> {
        let filters = if !self.filters.is_empty() || self.limit.is_some() || !self.joins.is_empty()
        {
            vec![QueryPredicate::is_in::<TEdge::TargetField, _>(self.select::<TEdge>()).to_expr()]
        } else {
            // Optimization: if the current query has no filters, joins, or limits, we can skip the subquery and just
            // query directly on the edge table
            vec![]
        };
        EntQuery {
            filters,
            joins: Vec::new(),
            limit: None,
            order: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Query an inbound edge (edge reference)
    pub fn query_edge_ref<TEdge: EntEdge>(self) -> EntQuery<TEdge::Ent>
    where
        TEdge::TargetField: EntField<Ent = TEnt>,
    {
        let filters = if !self.filters.is_empty() || self.limit.is_some() || !self.joins.is_empty()
        {
            vec![QueryPredicate::is_in::<TEdge, _>(self.select::<TEdge::TargetField>()).to_expr()]
        } else {
            // Optimization: if the current query has no filters, joins, or limits, we can skip the subquery and just
            // query directly on the edge table
            vec![]
        };
        EntQuery {
            filters,
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
    pub fn join<TEdge: EntEdge<Ent = TEnt>>(
        self,
    ) -> EntQuery<EntWithEdges<TEnt, (<TEdge::TargetField as EntField>::Ent, ())>> {
        let mut joins = self.joins;
        joins.push(JoinDef {
            table: <TEdge::TargetField as EntField>::Ent::TABLE_NAME,
            left_table: TEnt::TABLE_NAME,
            left_col: TEdge::NAME,
            right_table: <TEdge::TargetField as EntField>::Ent::TABLE_NAME,
            right_col: <TEdge::TargetField as EntField>::NAME,
        });
        EntQuery {
            filters: self.filters,
            joins,
            limit: self.limit,
            order: self.order,
            _marker: std::marker::PhantomData,
        }
    }

    /// Loads entities matching the query, applying privacy policies to filter results as needed.
    pub async fn load<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<Vec<TEnt>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let query_policy = TEnt::query_policy();

        let limit = self.limit;
        let mut select: sea_query::SelectStatement = self.into();

        let conn = &ctx.conn;

        let mut results = Vec::new();
        let mut offset = 0;
        'query: loop {
            let (sql, values) = select.build_sqlx(sea_query::PostgresQueryBuilder);
            let mut rows = sqlx::query_with(&sql, values).fetch(conn);

            // Evaluate privacy policies for each result, and only include results that pass.
            let mut result_count = 0;
            'rows: while let Some(row) = rows.next().await {
                result_count += 1;

                let ent = row
                    .map(|r| TEnt::from(&r))
                    .map_err(EntLoadError::DatabaseError)?;

                'rules: for rule in &query_policy {
                    match rule.evaluation(ctx, &ent).await {
                        PrivacyRuleOutcome::Allow => {
                            results.push(ent);

                            if let Some(limit) = limit
                                && results.len() >= limit
                            {
                                // We've loaded the desired number of results, so we can stop
                                break 'query;
                            }

                            continue 'rows;
                        }
                        PrivacyRuleOutcome::Deny => {
                            // This result did not pass the privacy policy, so don't include it in the results and move
                            // on to the next row
                            continue 'rows;
                        }
                        PrivacyRuleOutcome::Skip => {
                            // No determination for this rule, so process the next one.
                            continue 'rules;
                        }
                    }
                }
            }

            if let Some(limit) = limit {
                if result_count < limit {
                    // We've loaded all results, so we can stop
                    break 'query;
                }

                // We have not loaded the desired number of results, so we'll need to load more - update the offset and
                // run the query again
                //
                // TODO: dynamically adjust the limit to try to minimize the number of queries we need to run, and
                // consider putting an upper bound on the number of queries we will run to avoid full table scans.
                offset += limit as u64;
                select.offset(offset);
            } else {
                // We've already loaded all results, so we can stop
                break 'query;
            }
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

    /// Loads the first result, returning None if there are no results. Will not return an error if there are multiple
    /// results.
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
