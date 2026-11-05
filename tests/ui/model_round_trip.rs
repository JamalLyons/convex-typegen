#[path = "../../target/model-tests/models.rs"]
mod models;

use std::collections::BTreeMap;

use convex::Value;
use convex_typegen_runtime::ConvexValue;
use convex_typegen_runtime::function::{Action, FunctionReference, Mutation, Query, RawArgs};
use convex_typegen_runtime::model::{Doc, Id, RawValue};
use convex_typegen_runtime::value_serde::{Bytes, from_value, to_value};
use models::api::admin::users::EchoArgsDataNested;
use models::api::models::*;
use models::api::users::OptionalArgs;
use models::api::{admin, users};
use models::*;

fn object<const N: usize>(fields: [(&str, Value); N]) -> Value
{
    Value::Object(fields.into_iter().map(|(key, value)| (key.to_owned(), value)).collect())
}

fn round_trip<T>(wire: Value) -> T
where
    T: convex_typegen_runtime::serde::de::DeserializeOwned + convex_typegen_runtime::serde::Serialize,
{
    let model = from_value::<T>(wire.clone()).unwrap();
    assert_eq!(to_value(&model).unwrap(), wire);
    model
}

fn presence()
{
    let absent = object([("nested", object([]))]);
    let args = round_trip::<OptionalArgs>(absent);
    assert!(args.root.is_none() && args.nested.note.is_none());
    assert!(from_value::<OptionalArgs>(object([("nested", object([("note", Value::Null)]))])).is_err());

    let fields = [
        ("type", Value::Boolean(true)),
        ("9lives", Value::Int64(9)),
        ("self", Value::String("name".into())),
    ];
    let mut wire = object(fields.clone());
    let absent = round_trip::<RoundTripArgsCamelCase>(wire.clone());
    assert!(absent.some_key.is_none());
    assert!(absent.r#type);
    assert_eq!(absent._9lives, 9);
    assert_eq!(absent._self, "name");

    let Value::Object(ref mut fields) = wire else { unreachable!() };
    fields.insert("some-key".into(), Value::Null);
    assert!(matches!(
        round_trip::<RoundTripArgsCamelCase>(wire).some_key,
        Some(RoundTripArgsCamelCaseSomeKey::Branch2(()))
    ));
    assert!(from_value::<EchoArgsDataNested>(object([])).is_err());
    round_trip::<EchoArgsDataNested>(object([("nullable", Value::Null)]));
}

fn documents()
{
    let tasks = object([
        ("_id", Value::String("task".into())),
        ("_creationTime", Value::Float64(20.5)),
        ("text", Value::String("task".into())),
    ]);
    let doc: Doc<tables::Tasks> = round_trip(tasks.clone());
    assert_eq!(doc._id.as_str(), "task");
    assert_eq!(doc._creation_time, 20.5);
    let docs: Vec<Doc<tables::Tasks>> = round_trip::<admin::users::DocsReturn>(Value::Array(vec![tasks]));
    assert_eq!(docs.len(), 1);

    let event = |kind: &str, field| {
        object([
            ("_id", Value::String("event".into())),
            ("_creationTime", Value::Float64(20.5)),
            ("kind", Value::String(kind.into())),
            field,
        ])
    };
    assert!(matches!(
        round_trip::<Doc<tables::Events>>(event("created", ("id", Value::String("task".into())))),
        EventsDoc::Branch1(_)
    ));
    assert!(matches!(
        round_trip::<Doc<tables::Events>>(event("deleted", ("reason", Value::String("removed".into())))),
        EventsDoc::Branch2(_)
    ));
    assert!(from_value::<EventsDoc>(event("deleted", ("id", Value::String("task".into())))).is_err());
    assert!(from_value::<EventsDoc>(event("unknown", ("id", Value::String("task".into())))).is_err());
}

fn literals()
{
    use RoundTripArgsLiteral as L;
    assert!(matches!(round_trip::<L>(Value::String("1".into())), L::Branch1(_)));
    assert!(matches!(round_trip::<L>(Value::Float64(1.0)), L::Branch2(_)));
    assert!(matches!(round_trip::<L>(Value::Int64(1)), L::Branch3(_)));
    assert!(matches!(round_trip::<L>(Value::Boolean(true)), L::Branch4(_)));
    assert!(from_value::<L>(Value::Boolean(false)).is_err());
    assert!(from_value::<L>(Value::Int64(2)).is_err());
    assert!(from_value::<RoundTripArgsLiteralBranch2>(Value::Int64(1)).is_err());
    assert!(from_value::<RoundTripArgsLiteralBranch3>(Value::Float64(1.0)).is_err());
    assert!(from_value::<admin::users::PingReturn>(Value::String("ping".into())).is_err());
    round_trip::<admin::users::PingReturn>(Value::String("pong".into()));
}

fn kinds_and_records()
{
    use RoundTripArgsBuffer as B;
    use RoundTripReturn as N;
    assert!(matches!(round_trip::<N>(Value::Float64(1.0)), N::Branch1(1.0)));
    assert!(matches!(round_trip::<N>(Value::Int64(1)), N::Branch2(1)));
    assert!(matches!(round_trip::<B>(Value::Bytes(vec![0, 255])), B::Branch1(_)));
    assert!(matches!(round_trip::<B>(Value::Array(vec![Value::Int64(0)])), B::Branch2(_)));

    let mut wire = object([
        ("userId", Value::String("user".into())),
        (
            "camelCase",
            object([
                ("type", Value::Boolean(false)),
                ("9lives", Value::Int64(9)),
                ("self", Value::String("x".into())),
            ]),
        ),
        ("literal", Value::Boolean(true)),
        ("buffer", Value::Bytes(vec![42])),
        ("byKey", object([("two", Value::Bytes(vec![0, 255]))])),
        ("byId", object([("task", Value::Int64(i64::MAX))])),
        (
            "raw",
            object([
                ("bytes", Value::Bytes(vec![1])),
                ("n", Value::Int64(1)),
                ("f", Value::Float64(1.0)),
                ("null", Value::Null),
            ]),
        ),
    ]);
    let model = round_trip::<RoundTripArgs>(wire.clone());
    assert!(matches!(
        model.by_key.first_key_value().unwrap().0,
        RoundTripArgsByKeyKey::Branch2(_)
    ));
    assert_eq!(model.by_id.first_key_value().unwrap().0.as_str(), "task");
    assert!(matches!(model.raw, Some(RawValue(Value::Object(_)))));
    let Value::Object(ref mut fields) = wire else { unreachable!() };
    fields.insert("byKey".into(), object([("three", Value::Bytes(vec![]))]));
    assert!(from_value::<RoundTripArgs>(wire).is_err());
}

fn references()
{
    fn check<A, R, K>(reference: impl FunctionReference<Args = A, Returns = R, Kind = K>, path: &str)
    {
        assert_eq!(reference.path(), path);
    }

    check::<RawArgs, RawValue, Query>(users::list, "users:list");
    check::<admin::users::ListArgs, Vec<String>, Query>(admin::users::list, "admin/users:list");
    check::<admin::users::AddArgs, Id<tables::Tasks>, Mutation>(admin::users::add, "admin/users:add");
    check::<admin::users::FailArgs, (), Action>(admin::users::fail, "admin/users:fail");
    check::<api::r#type::_9_tools::ValueArgs, api::r#type::_9_tools::ValueReturn, Query>(
        api::r#type::_9_tools::value,
        "type/9-tools:value",
    );
    round_trip::<RawArgs>(object([("bytes", Value::Bytes(vec![255]))]));
    round_trip::<RawValue>(Value::Bytes(vec![255]));
}

#[derive(ConvexValue)]
struct HandwrittenArgs
{
    #[convex(rename = "userId")]
    user_id: Id<tables::Users>,
    #[convex(rename = "camelCase")]
    camel_case: HandwrittenNested,
    literal: HandwrittenLiteral,
    buffer: HandwrittenBuffer,
    #[convex(rename = "byKey")]
    by_key: BTreeMap<HandwrittenKey, Bytes>,
    #[convex(rename = "byId")]
    by_id: BTreeMap<Id<tables::Tasks>, i64>,
    #[convex(optional)]
    raw: Option<RawValue>,
}

#[derive(ConvexValue)]
struct HandwrittenNested
{
    #[convex(rename = "some-key", optional)]
    some_key: Option<HandwrittenNullable>,
    r#type: bool,
    #[convex(rename = "9lives")]
    lives: i64,
    #[convex(rename = "self")]
    name: String,
}

#[derive(ConvexValue)]
enum HandwrittenNullable
{
    Text(String),
    Null(()),
}

#[derive(ConvexValue)]
enum HandwrittenLiteral
{
    Text(OneText),
    Float(OneFloat),
    Int(OneInt),
    True(True),
}

#[derive(ConvexValue)]
#[convex(literal = "1")]
struct OneText;

#[derive(ConvexValue)]
#[convex(literal = 1.0)]
struct OneFloat;

#[derive(ConvexValue)]
#[convex(literal = 1)]
struct OneInt;

#[derive(ConvexValue)]
#[convex(literal = true)]
struct True;

#[derive(ConvexValue)]
enum HandwrittenBuffer
{
    Bytes(Bytes),
    Ints(Vec<i64>),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, ConvexValue)]
enum HandwrittenKey
{
    One(OneKey),
    Two(TwoKey),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, ConvexValue)]
#[convex(literal = "one")]
struct OneKey;

#[derive(PartialEq, Eq, PartialOrd, Ord, ConvexValue)]
#[convex(literal = "two")]
struct TwoKey;

#[derive(ConvexValue)]
#[convex(discriminator = "kind")]
enum HandwrittenEvent
{
    #[convex(tag = CreatedTag)]
    Created(Created),
    #[convex(tag = DeletedTag)]
    Deleted(Deleted),
}

#[derive(ConvexValue)]
#[convex(literal = "created")]
struct CreatedTag;

#[derive(ConvexValue)]
#[convex(literal = "deleted")]
struct DeletedTag;

#[derive(ConvexValue)]
struct Created
{
    _id: Id<tables::Events>,
    #[convex(rename = "_creationTime")]
    creation_time: f64,
    kind: CreatedTag,
    id: Id<tables::Tasks>,
}

#[derive(ConvexValue)]
struct Deleted
{
    _id: Id<tables::Events>,
    #[convex(rename = "_creationTime")]
    creation_time: f64,
    kind: DeletedTag,
    reason: String,
}

#[derive(ConvexValue)]
struct RequiredNullable
{
    value: Option<i64>,
}

#[derive(ConvexValue)]
#[convex(literal = -0.0)]
struct NegativeZero;

#[derive(ConvexValue)]
#[convex(literal = -9223372036854775808_i64)]
struct MinInt;

fn handwritten_models()
{
    for (literal, nullable) in [
        (Value::String("1".into()), None),
        (Value::Float64(1.0), Some(Value::Null)),
        (Value::Int64(1), Some(Value::String("present".into()))),
        (Value::Boolean(true), None),
    ] {
        let mut nested = BTreeMap::from([
            ("type".into(), Value::Boolean(false)),
            ("9lives".into(), Value::Int64(9)),
            ("self".into(), Value::String("x".into())),
        ]);
        if let Some(value) = nullable {
            nested.insert("some-key".into(), value);
        }
        let wire = object([
            ("userId", Value::String("user".into())),
            ("camelCase", Value::Object(nested)),
            ("literal", literal),
            ("buffer", Value::Bytes(vec![42])),
            ("byKey", object([("two", Value::Bytes(vec![0, 255]))])),
            ("byId", object([("task", Value::Int64(i64::MAX))])),
        ]);
        let hand = round_trip::<HandwrittenArgs>(wire);
        let generated = round_trip::<RoundTripArgs>(to_value(&hand).unwrap());
        round_trip::<HandwrittenArgs>(to_value(&generated).unwrap());
    }

    let event = object([
        ("_id", Value::String("event".into())),
        ("_creationTime", Value::Float64(20.5)),
        ("kind", Value::String("created".into())),
        ("id", Value::String("task".into())),
    ]);
    let generated = round_trip::<EventsDoc>(event);
    assert!(matches!(
        round_trip::<HandwrittenEvent>(to_value(&generated).unwrap()),
        HandwrittenEvent::Created(_)
    ));
    let invalid = object([("kind", Value::String("created".into())), ("reason", Value::Null)]);
    assert!(from_value::<HandwrittenEvent>(invalid.clone()).is_err());
    assert!(from_value::<EventsDoc>(invalid).is_err());
    assert!(from_value::<RequiredNullable>(object([])).is_err());
    round_trip::<RequiredNullable>(object([("value", Value::Null)]));
    assert!(from_value::<RequiredNullable>(object([("value", Value::Float64(1.0))])).is_err());
    assert!(from_value::<NegativeZero>(Value::Float64(0.0)).is_err());
    let negative = round_trip::<NegativeZero>(Value::Float64(-0.0));
    assert!(matches!(to_value(&negative).unwrap(), Value::Float64(v) if v.is_sign_negative()));
    round_trip::<MinInt>(Value::Int64(i64::MIN));
}

fn main()
{
    presence();
    documents();
    literals();
    kinds_and_records();
    references();
    handwritten_models();
}

#[allow(dead_code)]
async fn typed_calls(
    sdk: &mut convex::ConvexClient,
    args: admin::users::ListArgs,
) -> Result<(), convex_typegen_runtime::CallError<String>>
{
    use convex_typegen_runtime::TypedClient;

    let reference = admin::users::list.with_error::<String>();
    let updates = TypedClient::new(sdk).subscribe(reference, args.clone()).await?;
    let _: admin::users::ListReturn = TypedClient::new(sdk).query(reference, args).await?;
    drop(updates);
    Ok(())
}
