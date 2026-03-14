use std::collections::HashMap;

use crate::{
    Ent,
    field::{EntField, ReadWrite},
};

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
    field_mutations: HashMap<String, Box<dyn std::any::Any>>,
}

impl<'a, TEnt: Ent> EntMutator<'a, TEnt> {
    pub(crate) fn new(ent: &'a TEnt) -> Self {
        Self {
            ent,
            field_mutations: HashMap::new(),
        }
    }

    pub fn set<TField: EntField<Ent = TEnt, Visibility = ReadWrite>>(
        &mut self,
        new_value: TField::Value,
    ) {
        self.field_mutations
            .insert(TField::NAME.to_string(), Box::new(new_value));
    }

    pub fn unset<TField: EntField<Ent = TEnt, Visibility = ReadWrite>>(&mut self) {
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

#[cfg(test)]
mod tests {
    use crate::{self as resent, mutator};

    use super::*;

    #[derive(resent::EntSchema)]
    #[entschema(table = "test_ent")]
    pub struct TestEnt {
        #[field(readonly)]
        id: i32,
        value: String,
    }

    #[test]
    fn test_ent_mutator() {
        let ent = TestEnt {
            id: 1,
            value: "hello".to_string(),
        };

        let mut mutator = ent.mutate();
        assert_eq!(mutator.get::<test_ent::Id>().old, &1);

        mutator.set::<test_ent::Value>("world".to_string());
        assert_eq!(mutator.get::<test_ent::Value>().old, "hello");
        match mutator.get::<test_ent::Value>().new {
            EntMutationFieldState::Set(new_value) => assert_eq!(new_value, "world"),
            _ => panic!("Expected ValueField to be set"),
        }

        mutator.unset::<test_ent::Value>();
        assert_eq!(mutator.get::<test_ent::Value>().old, "hello");
        match mutator.get::<test_ent::Value>().new {
            EntMutationFieldState::Unset => (),
            _ => panic!("Expected ValueField to be unset"),
        }
    }
}
