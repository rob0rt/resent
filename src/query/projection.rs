use crate::{Ent, field::EntField, query::EntQuery};

pub struct EntFieldProjection<TField: EntField> {
    field: std::marker::PhantomData<TField>,
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent, TField: EntField<Ent = TEnt>>
    EntQuery<'ctx, Ctx, EntFieldProjection<TField>>
{
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent, TField: EntField<Ent = TEnt>>
    Into<sea_query::SelectStatement> for EntQuery<'ctx, Ctx, EntFieldProjection<TField>>
{
    fn into(self) -> sea_query::SelectStatement {
        let mut query = sea_query::Query::select();
        query.column((TField::Ent::TABLE_NAME, TField::NAME));
        query.from(TField::Ent::TABLE_NAME);
        query
    }
}
