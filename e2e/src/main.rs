use std::collections::BTreeMap;
use std::time::Duration;

use convex::{ConvexClient, Value};
use convex_typegen_runtime::model::{Doc, Id};
use convex_typegen_runtime::serde::Deserialize;
use convex_typegen_runtime::value_serde::{self, Bytes};
use convex_typegen_runtime::{CallError, TypedClient};
use futures::StreamExt;

#[allow(dead_code)]
mod generated
{
    convex_typegen_runtime::include!();
}
use generated::*;

const CALL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Deserialize)]
#[serde(crate = "convex_typegen_runtime::serde")]
struct Denied
{
    code: String,
    retry: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let url = std::env::var("CONVEX_SELF_HOSTED_URL")?;
    assert!(
        url.starts_with("http://127.0.0.1:"),
        "E2E must use a loopback self-hosted backend"
    );

    let generated = include_str!(concat!(env!("OUT_DIR"), "/convex_api.rs"));
    for forbidden in [
        "hiddenQuery",
        "hiddenMutation",
        "hiddenAction",
        "HttpAction",
        "crons:",
        "/ping",
    ] {
        assert!(
            !generated.contains(forbidden),
            "generated public API unexpectedly contains {forbidden}"
        );
    }
    assert!(generated.contains("admin/users:list"));
    assert!(generated.contains("users:list"));

    let mut sdk = tokio::time::timeout(CALL_TIMEOUT, ConvexClient::new(&url)).await??;
    let mut client = TypedClient::new(&mut sdk);

    let nested = tokio::time::timeout(
        CALL_TIMEOUT,
        client.query(api::admin::users::list, api::admin::users::ListArgs {}),
    )
    .await??;
    assert_eq!(nested, ["nested"]);
    let root = tokio::time::timeout(CALL_TIMEOUT, client.query(api::users::list, BTreeMap::new())).await??;
    assert!(matches!(root.0, Value::Array(ref values) if values == &[Value::String("root".into())]));

    let id: Id<tables::Tasks> = tokio::time::timeout(
        CALL_TIMEOUT,
        client.mutation(
            api::admin::users::add,
            api::admin::users::AddArgs {
                text: "typed e2e".into(),
            },
        ),
    )
    .await??;
    assert!(!id.as_str().is_empty());
    let docs: Vec<Doc<tables::Tasks>> = tokio::time::timeout(
        CALL_TIMEOUT,
        client.query(api::admin::users::docs, api::admin::users::DocsArgs {}),
    )
    .await??;
    assert!(docs.iter().any(|doc| doc._id == id && doc.text == "typed e2e"));

    let pong = tokio::time::timeout(
        CALL_TIMEOUT,
        client.action(api::admin::users::ping, api::admin::users::PingArgs {}),
    )
    .await??;
    assert_eq!(pong, api::admin::users::PingReturn);

    let error = api::admin::users::fail.with_error::<Denied>();
    match tokio::time::timeout(CALL_TIMEOUT, client.action(error, api::admin::users::FailArgs {})).await? {
        Err(CallError::Application {
            data: Ok(Denied { code, retry }),
            ..
        }) => {
            assert_eq!(code, "denied");
            assert!(!retry);
        }
        other => panic!("expected typed ConvexError payload, got {other:?}"),
    }

    let optional = api::users::OptionalArgs {
        root: None,
        nested: api::users::OptionalArgsNested { note: None },
    };
    let present = tokio::time::timeout(CALL_TIMEOUT, client.query(api::users::optional, optional)).await??;
    assert!(!present, "nested optional field must be omitted on the wire");

    let input = api::admin::users::EchoArgs {
        data: api::admin::users::EchoArgsData {
            bytes: Bytes(vec![0, 1, 127, 255]),
            choice: api::admin::users::EchoArgsDataChoice::Red,
            count: i64::MAX,
            nested: api::admin::users::EchoArgsDataNested {
                note: None,
                nullable: api::admin::users::EchoArgsDataNestedNullable::Branch2(()),
            },
            number: f64::from_bits(0x3fb9_9999_9999_999a),
            record: BTreeMap::from([("first".to_owned(), i64::MIN), ("last".to_owned(), i64::MAX)]),
        },
    };
    let echoed = tokio::time::timeout(CALL_TIMEOUT, client.query(api::admin::users::echo, input)).await??;
    assert_eq!(echoed.choice, api::admin::users::EchoReturnChoice::Red);
    assert_eq!(echoed.count, i64::MAX);
    assert_eq!(echoed.number.to_bits(), 0x3fb9_9999_9999_999a);
    assert_eq!(echoed.bytes.0, [0, 1, 127, 255]);
    assert_eq!(echoed.record["first"], i64::MIN);
    assert_eq!(echoed.record["last"], i64::MAX);
    assert!(echoed.nested.note.is_none());
    assert!(matches!(
        echoed.nested.nullable,
        api::admin::users::EchoReturnNestedNullable::Branch2(())
    ));

    let table_union = EventsDoc::Branch2(EventsDocBranch2 {
        _creation_time: 0.0,
        _id: Id::<tables::Events>::from("local:test".to_owned()),
        kind: EventsDocBranch2Kind,
        reason: "union round-trip".to_owned(),
    });
    let wire = value_serde::to_value(&table_union)?;
    assert!(matches!(wire, Value::Object(_)));
    assert!(matches!(value_serde::from_value::<EventsDoc>(wire)?, EventsDoc::Branch2(_)));

    let mut updates = tokio::time::timeout(
        CALL_TIMEOUT,
        client.subscribe(api::admin::users::docs, api::admin::users::DocsArgs {}),
    )
    .await??;
    let first = tokio::time::timeout(CALL_TIMEOUT, updates.next())
        .await?
        .expect("initial subscription value")?;
    assert!(first.iter().any(|doc| doc._id == id));
    let next_id = tokio::time::timeout(
        CALL_TIMEOUT,
        client.mutation(
            api::admin::users::add,
            api::admin::users::AddArgs {
                text: "subscription update".into(),
            },
        ),
    )
    .await??;
    let second = tokio::time::timeout(CALL_TIMEOUT, updates.next())
        .await?
        .expect("subsequent subscription value")?;
    assert!(second.iter().any(|doc| doc._id == next_id));
    assert_eq!(second.len(), first.len() + 1);

    println!("local self-hosted Convex E2E passed");
    Ok(())
}
