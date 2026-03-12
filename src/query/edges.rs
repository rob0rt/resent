use crate::{
    Ent, EntEdge,
    field::EntField,
    query::{EntQuery, JoinDef, predicate::FieldPredicate},
};

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
