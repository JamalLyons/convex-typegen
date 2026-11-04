//! Typed references to public Convex functions.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::model::RawValue;

/// Raw argument object for a function without a declared argument validator.
pub type RawArgs = BTreeMap<String, RawValue>;

/// Kind marker for a Convex query.
pub enum Query {}

/// Kind marker for a Convex mutation.
pub enum Mutation {}

/// Kind marker for a Convex action.
pub enum Action {}

/// Type information carried by a generated function reference.
pub trait FunctionReference
{
    type Args;
    type Returns;
    type Kind;

    /// Canonical Convex identifier, such as `admin/users:list`.
    fn path(&self) -> &'static str;
}

/// A public endpoint's canonical identifier and compile-time signature.
pub struct FunctionRef<Kind, Args, Returns>
{
    path: &'static str,
    signature: PhantomData<fn(Kind, Args) -> Returns>,
}

impl<Kind, Args, Returns> FunctionRef<Kind, Args, Returns>
{
    /// Construct a reference using the endpoint's canonical wire identifier.
    pub const fn new(path: &'static str) -> Self
    {
        Self {
            path,
            signature: PhantomData,
        }
    }

    /// Canonical Convex identifier, such as `admin/users:list`.
    pub const fn path(&self) -> &'static str
    {
        self.path
    }
}

impl<Kind, Args, Returns> Copy for FunctionRef<Kind, Args, Returns> {}

impl<Kind, Args, Returns> Clone for FunctionRef<Kind, Args, Returns>
{
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<Kind, Args, Returns> FunctionReference for FunctionRef<Kind, Args, Returns>
{
    type Args = Args;
    type Returns = Returns;
    type Kind = Kind;

    fn path(&self) -> &'static str
    {
        self.path()
    }
}
