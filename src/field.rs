use crate::{Ent, predicate::QueryPredicate};
use sea_query::Expr;

pub trait EntField<TEnt: Ent>: Sized {
    const NAME: &'static str;
    type Value: Into<Expr> + 'static;

    fn predicate(predicate: QueryPredicate<Self::Value>) -> EntFieldPredicate<TEnt, Self> {
        EntFieldPredicate {
            predicate: predicate.into(),
        }
    }
}

pub struct EntFieldPredicate<TEnt: Ent, TField: EntField<TEnt>> {
    predicate: QueryPredicate<TField::Value>,
}

impl<TEnt: Ent, TField: EntField<TEnt>> Into<Expr> for EntFieldPredicate<TEnt, TField> {
    fn into(self) -> Expr {
        self.predicate.to_expr(TField::NAME)
    }
}

/// A trait for getting the value of a field from an entity, used in both query predicates and mutation tracking.
pub trait EntFieldGetter<TEnt: Ent, T, TOut>
where
    Self: EntField<TEnt>,
{
    fn get(target: &T) -> &TOut;
}

/// A trait for setting the value of a field on an entity, used in mutation tracking.
pub trait EntFieldSetter<TEnt: Ent, T>
where
    Self: EntField<TEnt>,
{
    fn set(target: &mut T, new_value: Self::Value);
}
