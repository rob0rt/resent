use crate::{Ent, predicate::QueryPredicate};
use sea_query::Expr;

pub trait EntField<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>>: Sized {
    const NAME: &'static str;
    type Value: Into<Expr> + 'static;

    fn predicate(
        predicate: QueryPredicate<Self::Value>,
    ) -> EntFieldPredicate<'ctx, Ctx, TEnt, Self> {
        EntFieldPredicate {
            predicate: predicate.into(),
        }
    }
}

pub struct EntFieldPredicate<
    'ctx,
    Ctx: 'ctx + Sync,
    TEnt: Ent<'ctx, Ctx>,
    TField: EntField<'ctx, Ctx, TEnt>,
> {
    predicate: QueryPredicate<TField::Value>,
}

impl<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>, TField: EntField<'ctx, Ctx, TEnt>> Into<Expr>
    for EntFieldPredicate<'ctx, Ctx, TEnt, TField>
{
    fn into(self) -> Expr {
        self.predicate.to_expr(TField::NAME)
    }
}

/// A trait for getting the value of a field from an entity, used in both query predicates and mutation tracking.
pub trait EntFieldGetter<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>, T, TOut>
where
    Self: EntField<'ctx, Ctx, TEnt>,
{
    fn get(target: &T) -> &TOut;
}

/// A trait for setting the value of a field on an entity, used in mutation tracking.
pub trait EntFieldSetter<'ctx, Ctx: 'ctx + Sync, TEnt: Ent<'ctx, Ctx>, T>
where
    Self: EntField<'ctx, Ctx, TEnt>,
{
    fn set(target: &mut T, new_value: Self::Value);
}
