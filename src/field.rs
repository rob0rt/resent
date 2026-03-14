use crate::Ent;
use sea_query::Expr;

pub trait FieldVisibility {}
pub struct ReadOnly;
impl FieldVisibility for ReadOnly {}
pub struct ReadWrite;
impl FieldVisibility for ReadWrite {}

pub trait EntField: Sized {
    const NAME: &'static str;
    type Value: Into<Expr> + Clone + 'static;
    type Ent: Ent;
    type Visibility: FieldVisibility;

    fn get_value(ent: &Self::Ent) -> &Self::Value;
}

/// A trait for getting the value of a field from an entity, used in both query predicates and mutation tracking.
pub trait EntFieldGetter<T, TOut>
where
    Self: EntField,
{
    fn get(target: &T) -> &TOut;
}

/// A trait for setting the value of a field on an entity, used in mutation tracking.
pub trait EntFieldSetter<T>
where
    Self: EntField,
{
    fn set(target: &mut T, new_value: Self::Value);
}
