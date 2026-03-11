pub mod predicate;

use crate::{
    Ent, EntEdgeConfig,
    field::EntField,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
};
use predicate::FieldPredicate;
use sea_query::{Expr, SelectStatement};

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

pub struct EntQuery<'ctx, Ctx: 'ctx + Sync, TOut> {
    filters: Vec<Expr>,
    limit: Option<usize>,
    ctx: &'ctx QueryContext<Ctx>,
    _marker: std::marker::PhantomData<TOut>,
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent> EntQuery<'ctx, Ctx, TEnt> {
    pub fn new(ctx: &'ctx QueryContext<Ctx>) -> Self {
        Self {
            filters: Vec::new(),
            limit: None,
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

    pub fn query<TOtherEnt: Ent>(self) -> EntQuery<'ctx, Ctx, TOtherEnt>
    where
        TOtherEnt: EntEdgeConfig<TEnt>,
    {
        // Note: This is a placeholder implementation. The actual join logic would need to be implemented here.
        EntQuery {
            filters: self.filters,
            limit: self.limit,
            ctx: self.ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// A join will include a related entity in the query output
    pub fn join<TOtherEnt: Ent>(self) -> EntQuery<'ctx, Ctx, EntWithEdges<TEnt, (TOtherEnt, ())>>
    where
        TEnt: EntEdgeConfig<TOtherEnt>,
    {
        // Note: This is a placeholder implementation. The actual join logic would need to be implemented here.
        EntQuery {
            filters: self.filters,
            limit: self.limit,
            ctx: self.ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn load(self) -> Result<Vec<TEnt>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        let query_policy = TEnt::query_policy();

        let (ctx, select): (&QueryContext<Ctx>, SelectStatement) = self.into();
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
    pub fn where_field<TAnyEnt: Ent, TField: EntField<Ent = TEnt>, Index>(
        mut self,
        field_query: impl FieldPredicate<TField>,
    ) -> Self
    where
        (TEnt, TEdges): ContainsEnt<TAnyEnt, Index>,
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
        // Note: This is a placeholder implementation. The actual join logic would need to be implemented here.
        EntQuery {
            filters: self.filters,
            limit: self.limit,
            ctx: self.ctx,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn foo(self) -> EntWithEdges<TEnt, TEdges> {
        unimplemented!()
    }
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent> Into<(&'ctx QueryContext<Ctx>, sea_query::SelectStatement)>
    for EntQuery<'ctx, Ctx, TEnt>
{
    fn into(self) -> (&'ctx QueryContext<Ctx>, sea_query::SelectStatement) {
        // use sea_query::{Asterisk, ExprTrait, IntoIden, Query};
        let mut query = sea_query::Query::select();
        query
            .column(sea_query::Asterisk)
            .from(sea_query::Alias::new(TEnt::TABLE_NAME));
        for expr in self.filters {
            query.and_where(expr);
        }
        if let Some(limit) = self.limit {
            query.limit(limit as u64);
        }
        (self.ctx, query.to_owned())
    }
}

type Foo = EntWithEdges<String, (i32, (bool, ()))>;

fn test() {
    let foo = Foo {
        ent: "Hello".to_string(),
        edges: (42, (true, ())),
    };

    let i32: &i32 = foo.edge();
    let bool: &bool = foo.edge();
    // assert_eq!(*i32, 42);
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
