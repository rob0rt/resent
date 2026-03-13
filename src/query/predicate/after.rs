use sea_query::{Expr, ExprTrait, Value};

use crate::{
    Ent,
    field::EntField,
    query::predicate::{FieldPredicate, QueryPredicate},
};

pub trait AfterFieldValue: Into<Value> {}
impl AfterFieldValue for chrono::NaiveDateTime {}
impl AfterFieldValue for chrono::DateTime<chrono::Utc> {}

impl QueryPredicate {
    /// Creates a predicate that checks if a field's value is after the given value.
    /// Maps to a SQL `>` operator
    pub fn after<TValue: AfterFieldValue, T: EntField<Value = TValue>>(
        value: TValue,
    ) -> impl FieldPredicate<T> {
        struct AfterPredicate<TValue: AfterFieldValue>(TValue);

        impl<TValue: AfterFieldValue, T: EntField<Value = TValue>> FieldPredicate<T>
            for AfterPredicate<TValue>
        {
            fn to_expr(self) -> Expr {
                Expr::expr(Expr::col((T::Ent::TABLE_NAME, T::NAME))).gt(self.0)
            }
        }

        AfterPredicate(value)
    }
}
