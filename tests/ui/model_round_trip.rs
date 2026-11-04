#[path = "../../target/model-tests/models.rs"]
mod models;

use convex::Value;
use convex_typegen_runtime::function::{Action, FunctionReference, Mutation, Query, RawArgs};
use convex_typegen_runtime::model::{Doc, Id, RawValue};
use convex_typegen_runtime::value_serde::{from_value, to_value};
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

fn main()
{
    presence();
    documents();
    literals();
    kinds_and_records();
    references();
}
