use sea_query::{Expr, ExprTrait, Query, SelectStatement};

use crate::field::EntField;

pub trait FieldPredicate<TField: EntField> {
    fn to_expr(self) -> Expr;
}

pub struct QueryPredicate;

impl QueryPredicate {
    pub fn equals<T: EntField>(value: T::Value) -> impl FieldPredicate<T> {
        struct EqualsPredicate<T: EntField>(T::Value);

        impl<T: EntField> FieldPredicate<T> for EqualsPredicate<T> {
            fn to_expr(self) -> Expr {
                Expr::expr(Expr::col(T::NAME)).eq(self.0)
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
                Expr::expr(Expr::col(T::NAME)).gt(self.0)
            }
        }

        AfterPredicate(value)
    }
}

// pub enum QueryPredicate<T: Into<Expr>> {
//     Equals(T),
//     Not(Box<QueryPredicate<T>>),
//     In(Vec<T>),
//     InSubquery(SelectStatement),
// }

// impl<T: Into<Expr>> QueryPredicate<T> {
//     pub fn to_expr(self, col: &str) -> Expr {
//         match self {
//             QueryPredicate::Equals(value) => Expr::expr(Expr::col(col.to_string())).eq(value),
//             QueryPredicate::Not(inner) => {
//                 let inner_expr = inner.to_expr(col);
//                 Expr::not(inner_expr)
//             }
//             QueryPredicate::In(values) => Expr::col(col.to_string()).is_in(values),
//             QueryPredicate::InSubquery(subquery) => {
//                 Expr::col(col.to_string()).in_subquery(subquery)
//             }
//         }
//     }
// }
