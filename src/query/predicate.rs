use sea_query::{Expr, ExprTrait};

use crate::{
    Ent,
    field::EntField,
    query::{EntQuery, projection::EntFieldProjection},
};

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

    pub fn not<T: EntField>(predicate: impl FieldPredicate<T>) -> impl FieldPredicate<T> {
        struct NotPredicate(Expr);

        impl<T: EntField> FieldPredicate<T> for NotPredicate {
            fn to_expr(self) -> Expr {
                self.0.not()
            }
        }

        NotPredicate(predicate.to_expr())
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

    pub fn is_in<TField: EntField, T: InFieldExpression<TField>>(
        values: T,
    ) -> impl FieldPredicate<TField> {
        struct InPredicate<TField: EntField, T: InFieldExpression<TField>>(
            T,
            std::marker::PhantomData<TField>,
        );

        impl<TField: EntField, T: InFieldExpression<TField>> FieldPredicate<TField>
            for InPredicate<TField, T>
        {
            fn to_expr(self) -> Expr {
                self.0.is_in()
            }
        }

        InPredicate(values, std::marker::PhantomData)
    }
}

pub trait InFieldExpression<TField: EntField> {
    fn is_in(self) -> Expr;
}

impl<TField: EntField> InFieldExpression<TField> for Vec<TField::Value> {
    fn is_in(self) -> Expr {
        Expr::col(TField::NAME).is_in(self)
    }
}

impl<'ctx, Ctx: 'ctx + Sync, TField: EntField, TProjectedField: EntField<Value = TField::Value>>
    InFieldExpression<TField> for EntQuery<'ctx, Ctx, EntFieldProjection<TProjectedField>>
{
    fn is_in(self) -> Expr {
        Expr::col(TField::NAME).in_subquery(self.into())
    }
}
