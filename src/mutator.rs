use std::collections::{BTreeMap, HashMap};

use crate::{Ent, field::EntField, query::QueryContext};

struct EntMutator<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>> {
    entity: TEnt,
    mutations: HashMap<String, Box<dyn std::any::Any>>,
    _marker: std::marker::PhantomData<&'ctx Ctx>,
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>> EntMutator<'ctx, Ctx, TEnt> {
    fn new(entity: TEnt) -> Self {
        Self {
            entity,
            mutations: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn set<TField>(&mut self, new_value: TField::Value)
    where
        TField: EntField<'ctx, Ctx, TEnt> + 'static,
    {
        self.mutations
            .insert(TField::NAME.to_string(), Box::new(new_value));
    }

    fn get<TField>(&self) -> Option<&TField::Value>
    where
        TField: EntField<'ctx, Ctx, TEnt> + 'static,
    {
        self.mutations
            .get(TField::NAME)
            .and_then(|mutation| mutation.downcast_ref::<TField::Value>())
    }

    fn apply(self, context: &'ctx QueryContext<Ctx>) -> TEnt {
        // Here we would apply the mutations to the entity and save it to the database.
        // This is a placeholder implementation.
        self.entity
    }
}
