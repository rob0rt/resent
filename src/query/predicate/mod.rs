mod after;
mod is_in;

use crate::{Ent, field::EntField};
use sea_query::{Expr, ExprTrait};

pub trait FieldPredicate<TField: EntField> {
    fn to_expr(self) -> Expr;
}

pub struct QueryPredicate;

impl QueryPredicate {
    /// Creates a predicate that checks if a field's value is equal to the given value. Maps to a SQL `=` operator.
    pub fn equals<T: EntField>(value: T::Value) -> impl FieldPredicate<T> {
        struct EqualsPredicate<T: EntField>(T::Value);

        impl<T: EntField> FieldPredicate<T> for EqualsPredicate<T> {
            fn to_expr(self) -> Expr {
                Expr::expr(Expr::col((T::Ent::TABLE_NAME, T::NAME))).eq(self.0)
            }
        }

        EqualsPredicate(value)
    }

    /// Creates a predicate that negates another predicate, e.g. `NOT (predicate)`.
    pub fn not<T: EntField>(predicate: impl FieldPredicate<T>) -> impl FieldPredicate<T> {
        struct NotPredicate(Expr);

        impl<T: EntField> FieldPredicate<T> for NotPredicate {
            fn to_expr(self) -> Expr {
                self.0.not()
            }
        }

        NotPredicate(predicate.to_expr())
    }
}
