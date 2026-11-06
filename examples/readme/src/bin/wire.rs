// ANCHOR: wire
use std::collections::BTreeMap;

use convex::Value;
use convex_typegen_runtime::ConvexValue;
use convex_typegen_runtime::value_serde::{from_value, to_value};

#[derive(Debug, PartialEq, ConvexValue)]
enum Priority
{
    #[convex(literal = "low")]
    Low,
    #[convex(literal = "high")]
    High,
}

#[derive(Debug, PartialEq, ConvexValue)]
struct TaskPatch
{
    #[convex(rename = "taskText")]
    text: String,
    #[convex(optional)]
    priority: Option<Option<Priority>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    for (priority, wire_priority) in [
        (None, None),
        (Some(None), Some(Value::Null)),
        (Some(Some(Priority::High)), Some(Value::String("high".into()))),
    ] {
        let patch = TaskPatch {
            text: "Write Rust".into(),
            priority,
        };
        let mut expected = BTreeMap::from([("taskText".into(), Value::String("Write Rust".into()))]);
        if let Some(value) = wire_priority {
            expected.insert("priority".into(), value);
        }

        // Encode an SDK value, then decode it back into the handwritten model.
        let wire = to_value(&patch)?;
        assert_eq!(wire, Value::Object(expected));
        assert_eq!(from_value::<TaskPatch>(wire)?, patch);
    }
    Ok(())
}

// ANCHOR_END: wire

#[test]
fn wire_example_round_trips()
{
    main().unwrap();
}
