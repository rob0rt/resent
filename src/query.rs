use crate::{
    Ent,
    field::{EntField, EntFieldPredicate},
    predicate::EntQueryPredicate,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
};
use sea_query::{Expr, SelectStatement};

#[derive(Debug)]
pub enum EntLoadError {
    DatabaseError(sqlx::Error),
    NoResults,
    TooManyResults,
    InvalidPrivacyPolicy,
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

trait EntQueryFilter<TEnt: Ent>: Into<Expr> {}

pub struct EntQuery<'ctx, Ctx: 'ctx + Sync, TEnt: Ent> {
    // filters: Vec<Box<dyn EntQueryFilter<'ctx, Ctx, TEnt> + 'ctx>>,
    limit: Option<usize>,
    ctx: &'ctx QueryContext<Ctx>,
    _marker: std::marker::PhantomData<TEnt>,
}

trait QueryPredicate {}

impl<TEnt: Ent, TField: EntField<TEnt>> QueryPredicate for EntFieldPredicate<TEnt, TField> {}

impl<TEnt: Ent, T: Into<Expr>, TField: EntField<TEnt>> QueryPredicate
    for EntQueryPredicate<TEnt, TField, T>
{
}

trait EntEdge {}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent + EntPrivacyPolicy<'ctx, Ctx>> EntQuery<'ctx, Ctx, TEnt> {
    pub fn new(ctx: &'ctx QueryContext<Ctx>) -> Self {
        Self {
            // filters: Vec::new(),
            limit: None,
            ctx,
            _marker: std::marker::PhantomData,
        }
    }

    // pub fn filter<TField: EntField<'ctx, Ctx, TEnt>>(
    //     mut self,
    //     field_query: EntFieldPredicate<'ctx, Ctx, TEnt, TField>,
    // ) -> Self {
    //     self.field_queries.push(field_query.into());
    //     self
    // }

    pub fn filter<TPredicate: QueryPredicate>(mut self, predicate: TPredicate) -> Self {
        self
    }

    // pub fn filter<TEdge, TPredicate>(mut self, predicate: TPredicate) -> Self
    // where
    //     TEdge: EntEdge,
    //     TPredicate: QueryPredicate,
    // {
    //     self
    // }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub async fn load(self) -> Result<Vec<TEnt>, EntLoadError> {
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
    pub async fn load_only(self) -> Result<TEnt, EntLoadError> {
        let mut results = self.limit(2).load().await?;
        match results.len() {
            0 => Err(EntLoadError::NoResults),
            1 => Ok(results.remove(0)),
            _ => Err(EntLoadError::TooManyResults),
        }
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
        // for expr in self.field_queries {
        //     query.and_where(expr);
        // }
        if let Some(limit) = self.limit {
            query.limit(limit as u64);
        }
        (self.ctx, query.to_owned())
    }
}

// struct Here;
// struct There<T>(std::marker::PhantomData<T>);

// trait GetEdge<Edge, Index> {
//     fn edge(&self) -> &Edge;
// }

// impl<Edge, T> GetEdge<Edge, Here> for (Edge, T) {
//     fn edge(&self) -> &Edge {
//         &self.0
//     }
// }

// impl<Edge, H, T, Index> GetEdge<Edge, There<Index>> for (H, T)
// where
//     T: GetEdge<Edge, Index>,
// {
//     fn edge(&self) -> &Edge {
//         self.1.edge()
//     }
// }

// struct EntWithEdges<E, Edges> {
//     ent: E,
//     edges: Edges,
// }

// impl<E, Edges> EntWithEdges<E, Edges> {
//     fn edge<Edge, Index>(&self) -> &Edge
//     where
//         Edges: GetEdge<Edge, Index>,
//     {
//         self.edges.edge()
//     }
// }

// impl<E, Edges> std::ops::Deref for EntWithEdges<E, Edges> {
//     type Target = E;
//     fn deref(&self) -> &Self::Target {
//         &self.ent
//     }
// }
