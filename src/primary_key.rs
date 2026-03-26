use crate::{Ent, field::EntField};
use sea_query::{Expr, ExprTrait};
use std::hash::Hash;

pub trait EntPrimaryKey<TEnt: Ent> {
    type Value: Send + Sync + Clone + Hash + Eq + 'static;

    fn get_value(ent: &TEnt) -> Self::Value;

    fn as_expr(value: Self::Value) -> Expr;
}

impl<TEnt: Ent, TField: EntField<Ent = TEnt>> EntPrimaryKey<TEnt> for TField
where
    TField::Value: Hash + Eq,
{
    type Value = TField::Value;

    fn get_value(ent: &TEnt) -> Self::Value {
        TField::get_value(ent).clone()
    }

    fn as_expr(value: Self::Value) -> Expr {
        Expr::col((TEnt::TABLE_NAME, TField::NAME)).eq(value)
    }
}

impl<TEnt: Ent, T1: EntField<Ent = TEnt>, T2: EntField<Ent = TEnt>> EntPrimaryKey<TEnt> for (T1, T2)
where
    T1::Value: Hash + Eq,
    T2::Value: Hash + Eq,
{
    type Value = (T1::Value, T2::Value);

    fn get_value(ent: &TEnt) -> Self::Value {
        (T1::get_value(ent).clone(), T2::get_value(ent).clone())
    }

    fn as_expr(value: Self::Value) -> Expr {
        Expr::col((TEnt::TABLE_NAME, T1::NAME))
            .eq(value.0)
            .and(Expr::col((TEnt::TABLE_NAME, T2::NAME)).eq(value.1))
    }
}

impl<TEnt: Ent, T1: EntField<Ent = TEnt>, T2: EntField<Ent = TEnt>, T3: EntField<Ent = TEnt>>
    EntPrimaryKey<TEnt> for (T1, T2, T3)
where
    T1::Value: Hash + Eq,
    T2::Value: Hash + Eq,
    T3::Value: Hash + Eq,
{
    type Value = (T1::Value, T2::Value, T3::Value);

    fn get_value(ent: &TEnt) -> Self::Value {
        (
            T1::get_value(ent).clone(),
            T2::get_value(ent).clone(),
            T3::get_value(ent).clone(),
        )
    }

    fn as_expr(value: Self::Value) -> Expr {
        Expr::col((TEnt::TABLE_NAME, T1::NAME))
            .eq(value.0)
            .and(Expr::col((TEnt::TABLE_NAME, T2::NAME)).eq(value.1))
            .and(Expr::col((TEnt::TABLE_NAME, T3::NAME)).eq(value.2))
    }
}
