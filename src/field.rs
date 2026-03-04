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
