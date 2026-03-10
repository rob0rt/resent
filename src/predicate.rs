use sea_query::{Expr, ExprTrait, SelectStatement};

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
