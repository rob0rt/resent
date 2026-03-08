use crate::{
    Ent,
    field::{EntField, EntFieldGetter, EntFieldSetter},
};

pub enum EntMutationFieldState<
    'ctx,
    Ctx: 'ctx + Sync,
    TEnt: Ent<'ctx, Ctx>,
    TField: EntField<'ctx, Ctx, TEnt>,
> {
    /// The field is not being mutated.
    Unset,

    /// The field is being mutated to a new value.
    Set(Box<TField::Value>),
}

/// Represents a reference to a field mutation, allowing us to track the old and new values of the field.
pub struct EntMutationField<
    'a,
    'ctx,
    Ctx: 'ctx + Sync,
    TEnt: Ent<'ctx, Ctx>,
    TField: EntField<'ctx, Ctx, TEnt>,
> {
    old: &'a TField::Value,
    new: &'a EntMutationFieldState<'ctx, Ctx, TEnt, TField>,
}

pub trait EntMutator<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>>
where
    Self: Sized,
{
    fn set<TField: EntField<'ctx, Ctx, TEnt>>(&mut self, new_value: TField::Value)
    where
        TField: EntFieldSetter<'ctx, Ctx, TEnt, Self>,
    {
        TField::set(self, new_value);
    }

    fn get<'a, TField: EntField<'ctx, Ctx, TEnt>>(
        &'a self,
    ) -> EntMutationField<'a, 'ctx, Ctx, TEnt, TField>
    where
        TField: EntFieldGetter<'ctx, Ctx, TEnt, TEnt, <TField as EntField<'ctx, Ctx, TEnt>>::Value>,
        TField:
            EntFieldGetter<'ctx, Ctx, TEnt, Self, EntMutationFieldState<'ctx, Ctx, TEnt, TField>>,
    {
        EntMutationField {
            old: TField::get(self.get_ent()),
            new: TField::get(self),
        }
    }

    fn get_ent(&self) -> &TEnt;
}
