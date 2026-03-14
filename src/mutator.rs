use crate::{Ent, field::EntField, privacy::EntPrivacyPolicy, query::QueryContext};

pub enum EntMutationError {
    DatabaseError(sqlx::Error),
    PrivacyPolicyDenied,
}

pub enum EntMutationFieldState<'a, TField: EntField> {
    /// The field is not being mutated.
    Unset,

    /// The field is being mutated to a new value.
    Set(&'a TField::Value),
}

/// Represents a reference to a field mutation, allowing us to track the old and new values of the field.
pub struct EntMutationField<'a, TField: EntField> {
    pub old: &'a TField::Value,
    pub new: EntMutationFieldState<'a, TField>,
}

pub struct EntMutator<'a, TEnt: Ent> {
    ent: &'a TEnt,
    field_mutations: std::collections::HashMap<String, Box<dyn std::any::Any>>,
}

impl<'a, TEnt: Ent> EntMutator<'a, TEnt> {
    pub(crate) fn new(ent: &'a TEnt) -> Self {
        Self {
            ent,
            field_mutations: std::collections::HashMap::new(),
        }
    }

    pub fn set<TField: EntField<Ent = TEnt>>(&mut self, new_value: TField::Value) {
        self.field_mutations
            .insert(TField::NAME.to_string(), Box::new(new_value));
    }

    pub fn unset<TField: EntField<Ent = TEnt>>(&mut self) {
        self.field_mutations.remove(TField::NAME);
    }

    pub fn get<'b, TField: EntField<Ent = TEnt>>(&'b self) -> EntMutationField<'b, TField> {
        EntMutationField {
            old: TField::get_value(self.ent),
            new: match self.field_mutations.get(TField::NAME) {
                Some(boxed_value) => {
                    let new_value = boxed_value.downcast_ref::<TField::Value>().unwrap();
                    EntMutationFieldState::Set(new_value)
                }
                None => EntMutationFieldState::Unset,
            },
        }
    }

    // async fn apply<'ctx, Ctx: 'ctx + Sync>(
    //     self,
    //     ctx: &'ctx QueryContext<Ctx>,
    // ) -> Result<TEnt, EntMutationError>
    // where
    //     TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    // {
    //     // for (field_name, boxed_value) in self.field_mutations {}
    // }
}
