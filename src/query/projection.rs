use crate::{Ent, field::EntField, query::EntQuery};

pub struct EntFieldProjection<TField: EntField> {
    field: std::marker::PhantomData<TField>,
}

impl<TEnt: Ent, TField: EntField<Ent = TEnt>> EntQuery<EntFieldProjection<TField>> {}
