use crate::{
    Ent,
    field::{EntField, EntFieldGetter, EntFieldSetter},
    privacy::EntPrivacyPolicy,
};

pub enum EntMutationFieldState<TField: EntField> {
    /// The field is not being mutated.
    Unset,

    /// The field is being mutated to a new value.
    Set(Box<TField::Value>),
}

/// Represents a reference to a field mutation, allowing us to track the old and new values of the field.
pub struct EntMutationField<'a, TField: EntField> {
    old: &'a TField::Value,
    new: &'a EntMutationFieldState<TField>,
}

pub trait EntMutator<'ctx, Ctx: 'ctx + Sync, TEnt: Ent + EntPrivacyPolicy<'ctx, Ctx>>
where
    Self: Sized,
{
    fn set<TField: EntField>(&mut self, new_value: TField::Value)
    where
        TField: EntFieldSetter<Self>,
    {
        TField::set(self, new_value);
    }

    // fn get<'a, TField: EntField>(&'a self) -> EntMutationField<'a, TField>
    // where
    //     TField: EntFieldGetter<TField::Ent, <TField as EntField>::Value>,
    //     TField: EntFieldGetter<TField::Ent, Self, EntMutationFieldState<TField>>,
    // {
    //     EntMutationField {
    //         old: TField::get(self.get_ent()),
    //         new: TField::get(self),
    //     }
    // }

    fn get_ent(&self) -> &TEnt;
}
