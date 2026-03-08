use crate::{
    Ent,
    field::{EntField, EntFieldGetter, EntFieldSetter},
    privacy::EntPrivacyPolicy,
};

pub enum EntMutationFieldState<TEnt: Ent, TField: EntField<TEnt>> {
    /// The field is not being mutated.
    Unset,

    /// The field is being mutated to a new value.
    Set(Box<TField::Value>),
}

/// Represents a reference to a field mutation, allowing us to track the old and new values of the field.
pub struct EntMutationField<'a, TEnt: Ent, TField: EntField<TEnt>> {
    old: &'a TField::Value,
    new: &'a EntMutationFieldState<TEnt, TField>,
}

pub trait EntMutator<'ctx, Ctx: 'ctx + Sync, TEnt: Ent + EntPrivacyPolicy<'ctx, Ctx>>
where
    Self: Sized,
{
    fn set<TField: EntField<TEnt>>(&mut self, new_value: TField::Value)
    where
        TField: EntFieldSetter<TEnt, Self>,
    {
        TField::set(self, new_value);
    }

    fn get<'a, TField: EntField<TEnt>>(&'a self) -> EntMutationField<'a, TEnt, TField>
    where
        TField: EntFieldGetter<TEnt, TEnt, <TField as EntField<TEnt>>::Value>,
        TField: EntFieldGetter<TEnt, Self, EntMutationFieldState<TEnt, TField>>,
    {
        EntMutationField {
            old: TField::get(self.get_ent()),
            new: TField::get(self),
        }
    }

    fn get_ent(&self) -> &TEnt;
}
