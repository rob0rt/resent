use crate::{Ent, field::EntField, privacy::EntPrivacyPolicy, query::QueryContext};
use sea_query::{Expr, Query};
use sea_query_sqlx::SqlxBinder;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntCreatorError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Query error: {0}")]
    EntLoadError(#[from] sea_query::error::Error),
}

pub struct EntCreator<TEnt: Ent> {
    field_mutations: HashMap<String, (Box<dyn std::any::Any>, Expr)>,

    _marker: std::marker::PhantomData<TEnt>,
}

impl<TEnt: Ent> EntCreator<TEnt> {
    pub(crate) fn new() -> Self {
        Self {
            field_mutations: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the new value for a field in the mutation. This will overwrite any
    /// previous mutation for the same field.
    pub fn set<TField: EntField<Ent = TEnt>>(mut self, new_value: TField::Value) -> Self {
        let expr = new_value.clone().into();
        self.field_mutations
            .insert(TField::NAME.to_string(), (Box::new(new_value), expr));
        self
    }

    /// Unsets the value for a field in the mutation, effectively removing any
    /// previous mutation for that field.
    pub fn unset<TField: EntField<Ent = TEnt>>(mut self) -> Self {
        self.field_mutations.remove(TField::NAME);
        self
    }

    /// Applies the mutation by checking privacy policies, generating and executing the update statement, and reloading
    /// the updated entity.
    pub async fn apply<'ctx, Ctx: 'ctx + Sync>(
        self,
        ctx: &'ctx QueryContext<Ctx>,
    ) -> Result<TEnt, EntCreatorError>
    where
        TEnt: EntPrivacyPolicy<'ctx, Ctx>,
    {
        // Generate and execute the update statement
        let update_stmt: Result<sea_query::InsertStatement, sea_query::error::Error> = self.into();
        let update_stmt = update_stmt?;
        let (sql, args) = update_stmt.build_sqlx(sea_query::PostgresQueryBuilder);

        // Execute the update
        let query_result = sqlx::query_with(&sql, args)
            .fetch_one(&ctx.conn)
            .await
            .map_err(EntCreatorError::from)?;

        Ok(TEnt::from(&query_result))
    }
}

impl<TEnt: Ent> From<EntCreator<TEnt>>
    for Result<sea_query::InsertStatement, sea_query::error::Error>
{
    fn from(val: EntCreator<TEnt>) -> Self {
        let (columns, values): (Vec<_>, Vec<_>) = val
            .field_mutations
            .into_iter()
            .map(|(field_name, (_, expr))| (field_name, expr))
            .unzip();

        Ok(Query::insert()
            .into_table(TEnt::TABLE_NAME)
            .columns(columns)
            .values(values)?
            .returning_all()
            .to_owned())
    }
}
