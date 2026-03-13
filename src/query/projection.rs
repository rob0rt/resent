use crate::{Ent, field::EntField, query::EntQuery};

pub struct EntFieldProjection<TField: EntField> {
    field: std::marker::PhantomData<TField>,
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent, TField: EntField<Ent = TEnt>>
    EntQuery<'ctx, Ctx, EntFieldProjection<TField>>
{
}
