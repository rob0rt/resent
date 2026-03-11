use sea_query::{Expr, ExprTrait};

use crate::{Ent, field::EntField};

pub trait FieldPredicate<TField: EntField> {
    fn to_expr(self) -> Expr;
}

pub struct QueryPredicate;

impl QueryPredicate {
    pub fn equals<T: EntField>(value: T::Value) -> impl FieldPredicate<T> {
        struct EqualsPredicate<T: EntField>(T::Value);

        impl<T: EntField> FieldPredicate<T> for EqualsPredicate<T> {
            fn to_expr(self) -> Expr {
                Expr::expr(Expr::col((T::Ent::TABLE_NAME, T::NAME))).eq(self.0)
            }
        }

        EqualsPredicate(value)
    }

    pub fn after<T: EntField<Value = chrono::NaiveDateTime>>(
        value: chrono::NaiveDateTime,
    ) -> impl FieldPredicate<T> {
        struct AfterPredicate(chrono::NaiveDateTime);

        impl<T: EntField<Value = chrono::NaiveDateTime>> FieldPredicate<T> for AfterPredicate {
            fn to_expr(self) -> Expr {
                Expr::expr(Expr::col((T::Ent::TABLE_NAME, T::NAME))).gt(self.0)
            }
        }

        AfterPredicate(value)
    }

    // pub fn is_in<TField: EntField, T: InFieldExpression<TField>>(
    //     values: T,
    // ) -> impl FieldPredicate<TField> {
    //     // struct InPredicate<T: EntField>(Vec<T::Value>);

    //     // impl<T: EntField> FieldPredicate<T> for InPredicate<T> {
    //     //     fn to_expr(self) -> Expr {
    //     //         Expr::col(T::NAME).is_in(self.0)
    //     //     }
    //     // }

    //     // InPredicate(values)
    // }
}

trait InFieldExpression<TField: EntField> {
    fn in_field(self, field: TField) -> Expr;
}

impl<TField: EntField> InFieldExpression<TField> for Vec<TField::Value> {
    fn in_field(self, field: TField) -> Expr {
        Expr::col(TField::NAME).is_in(self)
    }
}
