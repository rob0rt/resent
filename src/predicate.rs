use sea_query::{Expr, ExprTrait, SelectStatement};

use crate::Ent;

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

pub enum QueryPredicate<T: Into<Expr>> {
    Equals(T),
    Not(Box<QueryPredicate<T>>),
    In(Vec<T>),
    InSubquery(SelectStatement),
}

impl<T: Into<Expr>> QueryPredicate<T> {
    pub fn to_expr(self, col: &str) -> Expr {
        match self {
            QueryPredicate::Equals(value) => Expr::expr(Expr::col(col.to_string())).eq(value),
            QueryPredicate::Not(inner) => {
                let inner_expr = inner.to_expr(col);
                Expr::not(inner_expr)
            }
            QueryPredicate::In(values) => Expr::col(col.to_string()).is_in(values),
            QueryPredicate::InSubquery(subquery) => {
                Expr::col(col.to_string()).in_subquery(subquery)
            }
        }
    }
}
