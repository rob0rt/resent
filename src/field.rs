use crate::{Ent, predicate::QueryPredicate};
use sea_query::Expr;

pub trait EntField: Sized {
    const NAME: &'static str;
    type Value: Into<Expr> + 'static;
    type Ent: Ent;

    fn predicate(predicate: QueryPredicate<Self::Value>) -> EntFieldPredicate<Self> {
        EntFieldPredicate {
            predicate: predicate.into(),
        }
    }
}

pub struct EntFieldPredicate<TField: EntField> {
    predicate: QueryPredicate<TField::Value>,
}

impl<TField: EntField> Into<Expr> for EntFieldPredicate<TField> {
    fn into(self) -> Expr {
        self.predicate.to_expr(TField::NAME)
    }
}

/// A trait for getting the value of a field from an entity, used in both query predicates and mutation tracking.
pub trait EntFieldGetter<T, TOut>
where
    Self: EntField,
{
    fn get(target: &T) -> &TOut;
}

/// A trait for setting the value of a field on an entity, used in mutation tracking.
pub trait EntFieldSetter<T>
where
    Self: EntField,
{
    fn set(target: &mut T, new_value: Self::Value);
}
