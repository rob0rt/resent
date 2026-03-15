use sea_query::Order;
use sea_query_sqlx::SqlxBinder;
use sqlx::postgres::PgRow;

use crate::{
    Ent, EntEdge,
    field::EntField,
    privacy::EntPrivacyPolicy,
    query::{
        EntLoadError, EntLoadOnlyError, EntQuery, JoinDef, QueryContext, predicate::FieldPredicate,
    },
};

pub struct EntWithEdges<E: Ent, Edges> {
    ent: E,
    edges: Edges,
}

impl<E: Ent, Edges> EntWithEdges<E, Edges> {
    pub fn edge<Edge, Index>(&self) -> &Edge
    where
        Edges: GetEdge<Edge, Index>,
    {
        self.edges.edge()
    }
}

impl<E: Ent, Edges> std::ops::Deref for EntWithEdges<E, Edges> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.ent
    }
}

impl<TEnt: Ent, TEdges: EdgeList> EntQuery<EntWithEdges<TEnt, TEdges>> {
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

    pub fn order_by<TField: EntField, Index>(mut self, dir: Order) -> Self
    where
        (TEnt, TEdges): ContainsEnt<TField::Ent, Index>,
    {
        self.order = Some((TField::NAME.to_string(), dir));
        self
    }

    pub fn join<TOtherEnt: Ent>(self) -> EntQuery<EntWithEdges<TEnt, (TOtherEnt, TEdges)>>
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

    /// Downcast the query to a specific entity type, as long as the new entity is contained in the edges. Note that
    /// this means only the privacy policy of the downcast-to entity will be applied.
    pub fn downcast<TTarget: Ent, Index>(self) -> EntQuery<TTarget>
    where
        (TEnt, TEdges): ContainsEnt<TTarget, Index>,
    {
        EntQuery {
            filters: self.filters,
            joins: self.joins,
            limit: self.limit,
            order: self.order,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn load<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<Vec<EntWithEdges<TEnt, TEdges>>, EntLoadError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
        TEdges: EdgeList,
    {
        let select: sea_query::SelectStatement = self.into();
        let (sql, values) = select.build_sqlx(sea_query::PostgresQueryBuilder);
        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&ctx.conn)
            .await
            .map_err(EntLoadError::DatabaseError)?;

        let mut results = Vec::new();
        for row in rows {
            let ent = TEnt::from(&row);
            let edges = TEdges::from_pg_row(&row);
            results.push(EntWithEdges { ent, edges });
        }
        Ok(results)
    }

    pub async fn only<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<EntWithEdges<TEnt, TEdges>, EntLoadOnlyError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
        TEdges: EdgeList,
    {
        let mut results = self.limit(2).load(ctx).await?;
        match results.len() {
            0 => Err(EntLoadOnlyError::NoResults),
            1 => Ok(results.remove(0)),
            _ => Err(EntLoadOnlyError::TooManyResults),
        }
    }
}

pub trait EdgeList {
    fn from_pg_row(row: &PgRow) -> Self;
}

impl<Edge: Ent> EdgeList for (Edge, ()) {
    fn from_pg_row(row: &PgRow) -> Self {
        (Edge::from(row), ())
    }
}

impl<Edge: Ent, T> EdgeList for (Edge, T)
where
    T: EdgeList,
{
    fn from_pg_row(row: &PgRow) -> (Edge, T) {
        (Edge::from(row), T::from_pg_row(row))
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
