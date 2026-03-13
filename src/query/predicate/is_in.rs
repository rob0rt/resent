use sea_query::{Expr, ExprTrait};

use crate::{
    Ent,
    field::EntField,
    query::{
        EntQuery,
        predicate::{FieldPredicate, QueryPredicate},
        projection::EntFieldProjection,
    },
};

pub trait InFieldExpression<TField: EntField> {
    fn is_in(self) -> Expr;
}

impl<TField: EntField> InFieldExpression<TField> for Vec<TField::Value> {
    fn is_in(self) -> Expr {
        Expr::col((TField::Ent::TABLE_NAME, TField::NAME)).is_in(self)
    }
}

impl<TField: EntField, TProjectedField: EntField<Value = TField::Value>> InFieldExpression<TField>
    for EntQuery<EntFieldProjection<TProjectedField>>
{
    fn is_in(self) -> Expr {
        let mut select: sea_query::SelectStatement = self.into();
        select
            .clear_selects()
            .column((TProjectedField::Ent::TABLE_NAME, TProjectedField::NAME));
        Expr::col((TField::Ent::TABLE_NAME, TField::NAME)).in_subquery(select)
    }
}

impl QueryPredicate {
    /// Creates a predicate that checks if a field's value is in the given list of values or subquery.
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
