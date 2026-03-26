use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::{Ent, primary_key::EntPrimaryKey};

#[derive(Default, Debug)]
pub struct EntCache {
    inner: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl EntCache {
    pub fn get<TEnt: Ent>(
        &self,
        pk: &<TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value,
    ) -> Option<TEnt> {
        let map = self.inner.read().unwrap();
        let type_map = map.get(&TypeId::of::<TEnt>())?;
        let typed_map = type_map
            .downcast_ref::<HashMap<<TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value, TEnt>>()?;
        typed_map.get(pk).cloned()
    }

    pub fn insert<TEnt: Ent>(
        &self,
        pk: <TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value,
        ent: TEnt,
    ) {
        let mut map = self.inner.write().unwrap();
        let type_map = map.entry(TypeId::of::<TEnt>()).or_insert_with(|| {
            Box::new(HashMap::<
                <TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value,
                TEnt,
            >::new())
        });
        let typed_map = type_map
            .downcast_mut::<HashMap<<TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value, TEnt>>()
            .unwrap();
        typed_map.insert(pk, ent);
    }

    pub fn invalidate<TEnt: Ent>(&self, pk: &<TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value) {
        let mut map = self.inner.write().unwrap();
        if let Some(type_map) = map.get_mut(&TypeId::of::<TEnt>())
            && let Some(typed_map) = type_map
                .downcast_mut::<HashMap<<TEnt::PrimaryKey as EntPrimaryKey<TEnt>>::Value, TEnt>>()
        {
            typed_map.remove(pk);
        }
    }
}
