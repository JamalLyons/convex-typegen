//! Typed references to public Convex functions.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use convex::Value;
use serde::de::DeserializeOwned;

use crate::client::InvalidApplicationData;
use crate::model::RawValue;
use crate::value_serde;

/// Raw argument object for a function without a declared argument validator.
pub type RawArgs = BTreeMap<String, RawValue>;

/// Kind marker for a Convex query.
pub enum Query {}

/// Kind marker for a Convex mutation.
pub enum Mutation {}

/// Kind marker for a Convex action.
pub enum Action {}

mod private
{
    pub trait Sealed {}

    impl<Kind, Args, Returns, Error> Sealed for super::FunctionRef<Kind, Args, Returns, Error> {}
}

/// Type information carried by a generated or handwritten [`FunctionRef`].
///
/// This trait is sealed. Use [`FunctionRef::new`] to describe a handwritten endpoint.
pub trait FunctionReference: private::Sealed
{
    /// Argument model passed to the typed client for this endpoint.
    type Args;
    /// Rust model decoded from a successful function result.
    type Returns;
    /// Marker that selects the typed client operation, such as [`Query`].
    type Kind;
    /// Application error payload decoded from a Convex function error.
    type Error;

    /// Decode the application payload, preserving the original value on failure.
    fn decode_error(&self, data: Value) -> Result<Self::Error, InvalidApplicationData>;

    /// Canonical Convex identifier, such as `admin/users:list`.
    fn path(&self) -> &'static str;
}

type ErrorDecoder<E> = fn(Value) -> Result<E, InvalidApplicationData>;

/// A public endpoint's canonical identifier and compile-time signature.
///
/// Generated code normally declares these references for you. They can also be constructed
/// directly when integrating a handwritten endpoint.
///
/// ```
/// use convex_typegen_runtime::function::{FunctionRef, Query};
///
/// let list = FunctionRef::<Query, (), Vec<String>>::new("tasks:list");
/// assert_eq!(list.path(), "tasks:list");
/// ```
pub struct FunctionRef<Kind, Args, Returns, Error = Value>
{
    path: &'static str,
    signature: PhantomData<fn(Kind, Args) -> Returns>,
    error_decoder: ErrorDecoder<Error>,
}

impl<Kind, Args, Returns> FunctionRef<Kind, Args, Returns>
{
    /// Construct a reference with no declared application error schema.
    pub const fn new(path: &'static str) -> Self
    {
        Self {
            path,
            signature: PhantomData,
            error_decoder: Ok,
        }
    }
}

impl<Kind, Args, Returns, Error> FunctionRef<Kind, Args, Returns, Error>
{
    /// Canonical Convex identifier, such as `admin/users:list`.
    pub const fn path(&self) -> &'static str
    {
        self.path
    }

    /// Declare the payload type used automatically by typed calls and subscriptions.
    pub const fn with_error<E: DeserializeOwned>(self) -> FunctionRef<Kind, Args, Returns, E>
    {
        FunctionRef {
            path: self.path,
            signature: PhantomData,
            error_decoder: decode_typed_error::<E>,
        }
    }

    /// Use the SDK's raw application error payload for this reference.
    pub const fn raw_errors(self) -> FunctionRef<Kind, Args, Returns>
    {
        FunctionRef::new(self.path)
    }
}

fn decode_typed_error<E: DeserializeOwned>(raw: Value) -> Result<E, InvalidApplicationData>
{
    value_serde::from_value(raw.clone()).map_err(|source| InvalidApplicationData { raw, source })
}

impl<Kind, Args, Returns, Error> Copy for FunctionRef<Kind, Args, Returns, Error> {}

impl<Kind, Args, Returns, Error> Clone for FunctionRef<Kind, Args, Returns, Error>
{
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<Kind, Args, Returns, Error> FunctionReference for FunctionRef<Kind, Args, Returns, Error>
{
    type Args = Args;
    type Returns = Returns;
    type Kind = Kind;
    type Error = Error;

    fn path(&self) -> &'static str
    {
        self.path()
    }

    fn decode_error(&self, data: Value) -> Result<Error, InvalidApplicationData>
    {
        (self.error_decoder)(data)
    }
}
