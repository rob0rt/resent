pub mod predicate;

use crate::{
    Ent, EntEdgeConfig,
    field::EntField,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
};
use predicate::FieldPredicate;
use sea_query::{ColumnRef, Expr, ExprTrait, SelectStatement};

#[derive(Debug)]
pub enum EntLoadError {
    DatabaseError(sqlx::Error),
    InvalidPrivacyPolicy,
}

#[derive(Debug)]
pub enum EntLoadOnlyError {
    LoadError(EntLoadError),
    NoResults,
    TooManyResults,
}

impl From<EntLoadError> for EntLoadOnlyError {
    fn from(value: EntLoadError) -> Self {
        EntLoadOnlyError::LoadError(value)
    }
}

#[derive(Clone)]
pub struct QueryContext<T> {
    conn: sqlx::PgPool,
    pub data: T,
}

impl<T> QueryContext<T> {
    pub fn new(conn: sqlx::PgPool, data: T) -> Self {
        Self { conn, data }
    }

    pub fn with<R>(&self, data: R) -> QueryContext<R> {
        QueryContext {
            conn: self.conn.clone(),
            data,
        }
    }
}

struct JoinDef {
    table: &'static str,
    left_table: &'static str,
    left_col: &'static str,
    right_table: &'static str,
    right_col: &'static str,
}

pub enum Ordering {
    Ascending,
    Descending,
}

impl Into<sea_query::Order> for Ordering {
    fn into(self) -> sea_query::Order {
        match self {
            Ordering::Ascending => sea_query::Order::Asc,
            Ordering::Descending => sea_query::Order::Desc,
        }
    }
}

pub struct EntQuery<'ctx, Ctx: 'ctx + Sync, TOut> {
    filters: Vec<Expr>,
    joins: Vec<JoinDef>,
    limit: Option<usize>,
    order: Option<(String, Ordering)>,
    ctx: &'ctx QueryContext<Ctx>,
    _marker: std::marker::PhantomData<TOut>,
}

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
        TOtherEnt: EntEdgeConfig<TEnt>,
    {
        let ctx = self.ctx;
        let mut subquery = sea_query::Query::select();
        subquery
            .column(sea_query::Alias::new(
                <TOtherEnt as EntEdgeConfig<TEnt>>::TargetField::NAME,
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
            <TOtherEnt as EntEdgeConfig<TEnt>>::SourceField::NAME,
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

    pub fn order_by<TField: EntField<Ent = TEnt>>(mut self, dir: Ordering) -> Self {
        self.order = Some((TField::NAME.to_string(), dir));
        self
    }

    /// A join will include a related entity in the query output
    pub fn join<TOtherEnt: Ent>(self) -> EntQuery<'ctx, Ctx, EntWithEdges<TEnt, (TOtherEnt, ())>>
    where
        TEnt: EntEdgeConfig<TOtherEnt>,
    {
        let mut joins = self.joins;
        joins.push(JoinDef {
            table: TOtherEnt::TABLE_NAME,
            left_table: TEnt::TABLE_NAME,
            left_col: <TEnt as EntEdgeConfig<TOtherEnt>>::SourceField::NAME,
            right_table: TOtherEnt::TABLE_NAME,
            right_col: <TEnt as EntEdgeConfig<TOtherEnt>>::TargetField::NAME,
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

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent, TEdges> EntQuery<'ctx, Ctx, EntWithEdges<TEnt, TEdges>> {
    pub fn where_field<TField: EntField, Index>(
        mut self,
        field_query: impl FieldPredicate<TField>,
    ) -> Self
    where
        (TEnt, TEdges): ContainsEnt<TField::Ent, Index>,
    {
        self.filters.push(field_query.to_expr());
        self
    }

    pub fn join<TOtherEnt: Ent>(
        self,
    ) -> EntQuery<'ctx, Ctx, EntWithEdges<TEnt, (TOtherEnt, TEdges)>>
    where
        TEnt: EntEdgeConfig<TOtherEnt>,
    {
        let mut joins = self.joins;
        joins.push(JoinDef {
            table: TOtherEnt::TABLE_NAME,
            left_table: TEnt::TABLE_NAME,
            left_col: <TEnt as EntEdgeConfig<TOtherEnt>>::SourceField::NAME,
            right_table: TOtherEnt::TABLE_NAME,
            right_col: <TEnt as EntEdgeConfig<TOtherEnt>>::TargetField::NAME,
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

pub struct Here;
pub struct There<T>(std::marker::PhantomData<T>);

pub trait GetEdge<Edge, Index> {
    fn edge(&self) -> &Edge;
}

impl<Edge, T> GetEdge<Edge, Here> for (Edge, T) {
    fn edge(&self) -> &Edge {
        &self.0
    }
}

impl<Edge, H, T, Index> GetEdge<Edge, There<Index>> for (H, T)
where
    T: GetEdge<Edge, Index>,
{
    fn edge(&self) -> &Edge {
        self.1.edge()
    }
}

pub trait ContainsEnt<E, Index> {}

impl<E, T> ContainsEnt<E, Here> for (E, T) {}

impl<E, H, T, Index> ContainsEnt<E, There<Index>> for (H, T) where T: ContainsEnt<E, Index> {}

pub struct EntWithEdges<E, Edges> {
    ent: E,
    edges: Edges,
}

impl<E, Edges> EntWithEdges<E, Edges> {
    pub fn edge<Edge, Index>(&self) -> &Edge
    where
        Edges: GetEdge<Edge, Index>,
    {
        self.edges.edge()
    }
}

impl<E, Edges> std::ops::Deref for EntWithEdges<E, Edges> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.ent
    }
}
