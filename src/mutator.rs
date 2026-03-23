use crate::{
    Ent,
    context::EntContext,
    field::{EntField, ReadWrite},
    primary_key::EntPrimaryKey,
    privacy::{EntPrivacyPolicy, PrivacyRuleOutcome},
    query::EntLoadOnlyError,
};
use sea_query::{Expr, Query};
use sea_query_sqlx::SqlxBinder;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntMutationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Privacy policy denied")]
    PrivacyPolicyDenied,
    #[error("Load error: {0}")]
    EntLoadError(#[from] EntLoadOnlyError),
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

    /// Maps field (column) names to the new value and the corresponding sea-query expression for that value. We store
    /// the new value as a boxed `Any` so that we can downcast it back to the correct type when inspecting the mutation.
    /// The sea-query expression is stored separately so that we can generate the SQL update statement without needing
    /// to know the concrete type of the value at that point.
    field_mutations: HashMap<String, (Box<dyn std::any::Any>, Expr)>,
}

impl<'a, TEnt: Ent> EntMutator<'a, TEnt> {
    pub(crate) fn new(ent: &'a TEnt) -> Self {
        Self {
            ent,
            field_mutations: HashMap::new(),
        }
    }

    /// Sets the new value for a field in the mutation. This will overwrite any
    /// previous mutation for the same field.
    pub fn set<TField: EntField<Ent = TEnt, Visibility = ReadWrite>>(
        &mut self,
        new_value: TField::Value,
    ) {
        let expr = new_value.clone().into();
        self.field_mutations
            .insert(TField::NAME.to_string(), (Box::new(new_value), expr));
    }

    /// Unsets the value for a field in the mutation, effectively removing any
    /// previous mutation for that field.
    pub fn unset<TField: EntField<Ent = TEnt, Visibility = ReadWrite>>(&mut self) {
        self.field_mutations.remove(TField::NAME);
    }

    /// Gets the current state of a field in the mutation, including the old
    /// value and the new value if it has been set.
    pub fn get<'b, TField: EntField<Ent = TEnt>>(&'b self) -> EntMutationField<'b, TField> {
        EntMutationField {
            old: TField::get_value(self.ent),
            new: match self.field_mutations.get(TField::NAME) {
                Some((boxed_value, _)) => {
                    let new_value = boxed_value.downcast_ref::<TField::Value>().unwrap();
                    EntMutationFieldState::Set(new_value)
                }
                None => EntMutationFieldState::Unset,
            },
        }
    }

    /// Applies the mutation by checking privacy policies, generating and executing the update statement, and reloading
    /// the updated entity.
    pub async fn apply<TCtx: EntContext>(self, ctx: &TCtx) -> Result<TEnt, EntMutationError>
    where
        TEnt: EntPrivacyPolicy<TCtx>,
    {
        // Get the primary key value of the entity being mutated - we'll need this to reload the entity after the
        // mutation is applied.
        let primary_key = TEnt::PrimaryKey::get_value(self.ent);

        // Check privacy policy
        let policies = TEnt::mutation_policy();
        for policy in policies {
            match policy.evaluation(ctx, self.ent).await {
                PrivacyRuleOutcome::Allow => (),
                PrivacyRuleOutcome::Deny => return Err(EntMutationError::PrivacyPolicyDenied),
                PrivacyRuleOutcome::Skip => continue,
            }
        }

        // Generate and execute the update statement
        let update_stmt: sea_query::UpdateStatement = self.into();
        let (sql, args) = update_stmt.build_sqlx(sea_query::PostgresQueryBuilder);

        // Execute the update
        sqlx::query_with(&sql, args)
            .execute(ctx.conn())
            .await
            .map_err(EntMutationError::from)?;

        // Reload and return the updated entity
        TEnt::load(ctx, primary_key)
            .await
            .map_err(EntMutationError::from)
    }
}

impl<TEnt: Ent> From<EntMutator<'_, TEnt>> for sea_query::UpdateStatement {
    fn from(val: EntMutator<'_, TEnt>) -> Self {
        Query::update()
            .table(TEnt::TABLE_NAME)
            .and_where(TEnt::PrimaryKey::as_expr(TEnt::PrimaryKey::get_value(
                val.ent,
            )))
            .values(
                val.field_mutations
                    .into_iter()
                    .map(|(field_name, (_, expr))| (field_name, expr))
                    .collect::<Vec<_>>(),
            )
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as resent};

    use super::*;

    #[derive(resent::EntSchema)]
    #[entschema(table = "test_ent")]
    pub struct TestEnt {
        #[field(readonly, primary_key)]
        id: i32,
        value: String,
    }

    #[test]
    fn test_ent_mutation_field_state() {
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

    #[test]
    fn test_ent_mutator_into_update_statement() {
        let ent = TestEnt {
            id: 1,
            value: "hello".to_string(),
        };

        let mut mutator = ent.mutate();
        mutator.set::<test_ent::Value>("world".to_string());

        let update_stmt: sea_query::UpdateStatement = mutator.into();
        let (sql, args) = update_stmt.build_sqlx(sea_query::PostgresQueryBuilder);

        assert_eq!(
            sql,
            r#"UPDATE "test_ent" SET "value" = $1 WHERE "test_ent"."id" = $2"#
        );
        let args = args.0.0;
        assert_eq!(
            args,
            vec![
                sea_query::Value::String(Some("world".to_string())),
                sea_query::Value::Int(Some(1))
            ],
        );
    }
}
