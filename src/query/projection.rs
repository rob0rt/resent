use crate::{
    Ent,
    context::EntContext,
    field::EntField,
    privacy::EntPrivacyPolicy,
    query::{EntLoadError, EntLoadOnlyError, EntQuery},
};

pub struct EntFieldProjection<TField: EntField>(std::marker::PhantomData<TField>);

impl<TEnt: Ent, TField: EntField<Ent = TEnt>> EntQuery<EntFieldProjection<TField>> {
    pub async fn load<TCtx: EntContext>(
        self,
        ctx: &TCtx,
    ) -> Result<Vec<TField::Value>, EntLoadError>
    where
        TField::Ent: EntPrivacyPolicy<TCtx>,
    {
        self.downcast().load(ctx).await.map(|ents| {
            ents.into_iter()
                .map(|ent| TField::get_value(&ent).to_owned())
                .collect()
        })
    }

    pub async fn only<TCtx: EntContext>(self, ctx: &TCtx) -> Result<TField::Value, EntLoadOnlyError>
    where
        TField::Ent: EntPrivacyPolicy<TCtx>,
    {
        self.downcast()
            .only(ctx)
            .await
            .map(|ent| TField::get_value(&ent).to_owned())
    }

    pub async fn first<TCtx: EntContext>(
        self,
        ctx: &TCtx,
    ) -> Result<Option<TField::Value>, EntLoadError>
    where
        TField::Ent: EntPrivacyPolicy<TCtx>,
    {
        Ok(self
            .downcast()
            .first(ctx)
            .await?
            .map(|ent| TField::get_value(&ent).to_owned()))
    }

    /// Downcast the projection to the entity type. This is useful for querying
    /// since we want to load the full entities to apply privacy policies, but
    /// filter the results down to a specific field after loading.
    fn downcast(self) -> EntQuery<TField::Ent> {
        EntQuery {
            filters: self.filters,
            joins: self.joins,
            limit: self.limit,
            order: self.order,
            _marker: std::marker::PhantomData,
        }
    }
}
