//! Typed calls over the official Convex client.

use std::collections::BTreeMap;
use std::fmt;

use convex::{ConvexClient, FunctionResult, Value};
use futures::{Stream, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::function::{Action, FunctionReference, Mutation, Query};
use crate::value_serde;

/// A failure while calling a Convex function or decoding a subscription item.
///
/// Include a fallback arm when matching this enum. Use a struct pattern with `..`
/// for any variant, including tuple variants.
///
/// ```
/// use convex_typegen_runtime::CallError;
///
/// fn is_transport<E>(error: &CallError<E>) -> bool
/// {
///     matches!(error, CallError::Transport { .. })
/// }
/// ```
///
/// See [handling application errors](https://ctg.jamallyons.com/errors.html).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CallError<E>
{
    /// Arguments could not be encoded as a Convex object.
    #[error("could not encode Convex arguments: {0}")]
    #[non_exhaustive]
    Encode(#[source] value_serde::Error),
    /// A successful function result did not match the return type.
    #[error("could not decode Convex return value: {0}")]
    #[non_exhaustive]
    Decode(#[source] value_serde::Error),
    /// The SDK could not complete the request or establish the subscription.
    #[error("Convex client request failed: {0}")]
    #[non_exhaustive]
    Transport(#[source] TransportError),
    /// Convex returned a function error message without application data.
    #[error("{0}")]
    #[non_exhaustive]
    Server(String),
    /// The function threw a ConvexError with application data.
    #[error("{message}")]
    #[non_exhaustive]
    Application
    {
        /// The message supplied by Convex.
        message: String,
        /// The decoded payload or the original value and its decoding error.
        data: Result<E, InvalidApplicationData>,
    },
}

/// A failure from the SDK client or its query subscription.
#[derive(Debug)]
pub struct TransportError(anyhow::Error);

impl fmt::Display for TransportError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for TransportError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        Some(self.0.as_ref())
    }
}

/// A ConvexError payload that did not match the declared Rust type.
#[derive(Debug, thiserror::Error)]
#[error("could not decode Convex application error data: {source}")]
#[non_exhaustive]
pub struct InvalidApplicationData
{
    /// The original SDK value, with its wire kinds intact.
    pub raw: Value,
    /// The path-aware payload decoding failure.
    #[source]
    pub source: value_serde::Error,
}

/// A typed view of an existing SDK client.
///
/// The adapter borrows the client mutably for the same operations as the SDK.
/// Build and authenticate the SDK client directly or use [`Self::client_mut`]
/// while the adapter is in scope.
///
/// ```no_run
/// use convex_typegen_runtime::TypedClient;
///
/// fn adapter<'a>(client: &'a mut convex::ConvexClient) -> TypedClient<'a>
/// {
///     TypedClient::new(client)
/// }
/// ```
pub struct TypedClient<'a>
{
    client: &'a mut ConvexClient,
}

impl<'a> TypedClient<'a>
{
    /// Borrow an SDK client for typed operations.
    pub fn new(client: &'a mut ConvexClient) -> Self
    {
        Self { client }
    }

    /// Access the SDK client, including its authentication methods.
    pub fn client_mut(&mut self) -> &mut ConvexClient
    {
        self.client
    }

    /// Run a query once.
    pub async fn query<F>(&mut self, function: F, args: F::Args) -> Result<F::Returns, CallError<F::Error>>
    where
        F: FunctionReference<Kind = Query>,
        F::Args: Serialize,
        F::Returns: DeserializeOwned,
    {
        let mut subscription = self.subscribe(function, args).await?;
        subscription.next().await.unwrap_or_else(|| {
            Err(CallError::Transport(TransportError(anyhow::anyhow!(
                "Convex query subscription closed before its first result"
            ))))
        })
    }

    /// Run a mutation.
    pub async fn mutation<F>(&mut self, function: F, args: F::Args) -> Result<F::Returns, CallError<F::Error>>
    where
        F: FunctionReference<Kind = Mutation>,
        F::Args: Serialize,
        F::Returns: DeserializeOwned,
    {
        let result = self
            .client
            .mutation(function.path(), encode_args(args).map_err(CallError::Encode)?)
            .await;
        decode_result(&function, result)
    }

    /// Run an action.
    pub async fn action<F>(&mut self, function: F, args: F::Args) -> Result<F::Returns, CallError<F::Error>>
    where
        F: FunctionReference<Kind = Action>,
        F::Args: Serialize,
        F::Returns: DeserializeOwned,
    {
        let result = self
            .client
            .action(function.path(), encode_args(args).map_err(CallError::Encode)?)
            .await;
        decode_result(&function, result)
    }

    /// Subscribe to a query and decode each update independently.
    ///
    /// The stream owns the SDK subscription and releases it on drop. The
    /// mutable client borrow for setup ends when this method returns. Keep
    /// the SDK client alive while consuming updates.
    pub async fn subscribe<F>(
        &mut self,
        function: F,
        args: F::Args,
    ) -> Result<impl Stream<Item = Result<F::Returns, CallError<F::Error>>> + Unpin + use<F>, CallError<F::Error>>
    where
        F: FunctionReference<Kind = Query>,
        F::Args: Serialize,
        F::Returns: DeserializeOwned,
    {
        let subscription = self
            .client
            .subscribe(function.path(), encode_args(args).map_err(CallError::Encode)?)
            .await
            .map_err(|error| CallError::Transport(TransportError(error)))?;
        Ok(subscription.map(move |result| decode_result(&function, Ok(result))))
    }
}

fn encode_args(args: impl Serialize) -> Result<BTreeMap<String, Value>, value_serde::Error>
{
    match value_serde::to_value(&args)? {
        Value::Object(map) => Ok(map),
        _ => Err(value_serde::Error::expected_object()),
    }
}

fn decode_result<F>(function: &F, result: anyhow::Result<FunctionResult>) -> Result<F::Returns, CallError<F::Error>>
where
    F: FunctionReference,
    F::Returns: DeserializeOwned,
{
    match result.map_err(|error| CallError::Transport(TransportError(error)))? {
        FunctionResult::Value(value) => value_serde::from_value(value).map_err(CallError::Decode),
        FunctionResult::ErrorMessage(message) => Err(CallError::Server(message)),
        FunctionResult::ConvexError(error) => Err(CallError::Application {
            message: error.message,
            data: function.decode_error(error.data),
        }),
    }
}

#[cfg(test)]
mod tests
{
    use serde::Serialize;

    use super::*;
    use crate::function::{FunctionRef, RawArgs};
    use crate::value_serde::Bytes;

    #[derive(Serialize)]
    struct Args
    {
        integer: i64,
        float: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        omitted: Option<bool>,
        bytes: Bytes,
    }

    #[test]
    fn encode_args_preserves_wire_kinds_and_optional_omission()
    {
        let args = Args {
            integer: 9,
            float: 2.0,
            omitted: None,
            bytes: Bytes(vec![1, 2, 3]),
        };
        let map = encode_args(args).unwrap();

        assert!(matches!(map.get("integer"), Some(convex::Value::Int64(9))));
        assert!(matches!(map.get("float"), Some(convex::Value::Float64(2.0))));
        assert!(matches!(map.get("bytes"), Some(convex::Value::Bytes(bytes)) if bytes == &vec![1, 2, 3]));
        assert!(!map.contains_key("omitted"));
    }

    #[test]
    fn encode_args_rejects_non_object_root()
    {
        let error = encode_args("not args").unwrap_err();
        assert_eq!(error.path(), "$");
    }

    const QUERY: FunctionRef<Query, RawArgs, i64> = FunctionRef::new("test:query");

    #[derive(Debug, serde::Deserialize)]
    struct Failure
    {
        code: String,
    }

    const TYPED_QUERY: FunctionRef<Query, RawArgs, i64, Failure> = QUERY.with_error();

    fn application_error(data: Value) -> FunctionResult
    {
        FunctionResult::ConvexError(convex::ConvexError {
            message: "request rejected".into(),
            data,
        })
    }

    #[test]
    fn maps_sdk_results()
    {
        assert_eq!(
            decode_result(&QUERY, Ok(FunctionResult::Value(Value::Int64(42)))).unwrap(),
            42
        );
        assert!(matches!(
            decode_result(&QUERY, Ok(FunctionResult::Value(Value::Float64(42.0)))),
            Err(CallError::Decode { 0: error, .. }) if error.path() == "$"
        ));
        assert!(matches!(
            decode_result(&QUERY, Ok(FunctionResult::ErrorMessage("server failure".into()))),
            Err(CallError::Server { 0: message, .. }) if message == "server failure"
        ));
        assert!(matches!(
            decode_result(&QUERY, Err(anyhow::anyhow!("connection closed"))),
            Err(CallError::Transport { 0: error, .. }) if error.to_string() == "connection closed"
        ));
    }

    #[test]
    fn transport_error_preserves_display_and_source_chain()
    {
        use std::error::Error;
        use std::io;

        let sdk_error = anyhow::Error::new(io::Error::from(io::ErrorKind::ConnectionReset)).context("connection closed");
        let expected_message = sdk_error.to_string();
        let error = decode_result(&QUERY, Err(sdk_error)).unwrap_err();

        assert_eq!(error.to_string(), "Convex client request failed: connection closed");
        let transport = error.source().unwrap().downcast_ref::<TransportError>().unwrap();
        assert_eq!(transport.to_string(), expected_message);
        let sdk_context = transport.source().unwrap();
        assert_eq!(sdk_context.to_string(), expected_message);
        let cause = sdk_context.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(cause.kind(), io::ErrorKind::ConnectionReset);
        assert!(cause.source().is_none());
    }

    #[test]
    fn decodes_declared_application_error_and_supports_raw_override()
    {
        let raw = Value::Object(BTreeMap::from([("code".into(), Value::String("denied".into()))]));
        let result = application_error(raw.clone());
        assert!(matches!(
            decode_result(&TYPED_QUERY, Ok(result.clone())),
            Err(CallError::Application { message, data: Ok(Failure { code }) })
                if message == "request rejected" && code == "denied"
        ));
        assert!(matches!(
            decode_result(&TYPED_QUERY.raw_errors(), Ok(result)),
            Err(CallError::Application { data: Ok(value), .. }) if value == raw
        ));
    }

    #[test]
    fn keeps_raw_application_data_and_decode_error_when_payload_is_malformed()
    {
        let raw = Value::Object(BTreeMap::from([("code".into(), Value::Bytes(vec![0xff, 0xfe]))]));
        assert!(matches!(
            decode_result(&TYPED_QUERY, Ok(application_error(raw.clone()))),
            Err(CallError::Application {
                message,
                data: Err(InvalidApplicationData { raw: value, source }),
            }) if message == "request rejected" && value == raw && source.path() == "$.code"
        ));
    }
}
