# Handle application errors

Use a thrown `ConvexError` for a failure your app can act on, such as an invalid
form submission. In the [task tutorial](getting-started.md), the `tasks:add`
mutation throws `new ConvexError({ code: "empty_text" })` for a blank title.

## Decode a payload

Function validators and specs do not describe thrown payloads. Generated references
use the SDK's `convex::Value` for application error data by default. Declare a known
payload shape using `with_error::<T>()`, where `T` implements owned deserialization.
The tutorial uses a handwritten `AddFailure` with `#[derive(ConvexValue)]`, calls
`client.mutation(tasks::add.with_error::<AddFailure>(), invalid)`, and matches
`Err(CallError::Application { data: Ok(failure), .. })` to read `failure.code`.

The override applies to calls and subscription items that use that reference.
Use `raw_errors()` on a typed reference to return to raw SDK payloads, for example
when debugging an endpoint with multiple payload shapes.

## Choose a response

| Failure | What it means |
| --- | --- |
| `Application { data: Ok(payload), .. }` | The server threw `ConvexError` and its payload decoded. Use the payload to decide what the app should do. |
| `Application { data: Err(invalid), .. }` | The payload did not match the Rust type. Inspect `invalid.raw` and the path-aware `invalid.source`; the original failure is still an application error. |
| `Encode` | Arguments could not be encoded. Check your arguments and wire types. |
| `Decode` | A successful response did not match the expected return type. Check that your bindings and deployment agree. |
| `Transport` | The SDK could not complete the request or establish the subscription. Check the deployment URL and connection. |
| `Server` | Convex returned a plain error message without an application payload. Check the backend logs. |

`CallError` is non-exhaustive. Use a fallback arm when matching it and `..`
when destructuring application errors. Avoid logging raw payloads that may contain
user data.
