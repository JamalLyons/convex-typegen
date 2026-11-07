//! Serde building blocks for generated Convex models.
//! See [generated model names and wire behavior](https://ctg.jamallyons.com/docs/generated-models.html).

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use convex::Value;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A Convex ID with its table tracked at compile time.
///
/// Construct from a string only when its table is known. Convex validates the
/// actual ID and table on the server.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent, bound = "")]
pub struct Id<Table>(String, #[serde(skip)] PhantomData<fn() -> Table>);

impl<Table> From<String> for Id<Table>
{
    fn from(value: String) -> Self
    {
        Self(value, PhantomData)
    }
}

impl<Table> Id<Table>
{
    /// Borrow the wire ID for display or for an SDK call that takes a string.
    pub fn as_str(&self) -> &str
    {
        &self.0
    }

    /// Consume the typed ID to recover its owned wire string.
    pub fn into_string(self) -> String
    {
        self.0
    }
}

/// Associates a generated table marker with its full document model.
pub trait Table
{
    /// The table's full document model, including Convex's system fields.
    /// Use [`Doc<T>`] to name it from a table marker.
    type Document;
}

/// Full Convex document for a generated table marker.
pub type Doc<T> = <T as Table>::Document;

/// Decode a field that is present, even if its value is null.
/// Pair with Serde's `default` and `skip_serializing_if = "Option::is_none"`.
pub fn deserialize_present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// A raw SDK value with lossless Serde support through the Convex value codec.
/// Generated bindings use it for `v.any()` and undeclared validators.
#[derive(Debug, Clone, PartialEq)]
pub struct RawValue(pub Value);

impl Serialize for RawValue
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        RawValueRef(&self.0).serialize(serializer)
    }
}

struct RawValueRef<'a>(&'a Value);

impl Serialize for RawValueRef<'_>
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        use serde::ser::{SerializeMap, SerializeSeq};

        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Int64(value) => serializer.serialize_i64(*value),
            Value::Float64(value) => serializer.serialize_f64(*value),
            Value::Boolean(value) => serializer.serialize_bool(*value),
            Value::String(value) => serializer.serialize_str(value),
            Value::Bytes(value) => serializer.serialize_bytes(value),
            Value::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&RawValueRef(value))?;
                }
                seq.end()
            }
            Value::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (key, value) in fields {
                    map.serialize_entry(key, &RawValueRef(value))?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for RawValue
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        deserializer.deserialize_any(RawVisitor)
    }
}

struct RawVisitor;

impl<'de> Visitor<'de> for RawVisitor
{
    type Value = RawValue;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.write_str("a Convex wire value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Null))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Int64(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Float64(value)))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Boolean(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::String(value.into())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::String(value)))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Bytes(value.into())))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
    {
        Ok(RawValue(Value::Bytes(value)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error>
    {
        let mut values = Vec::new();
        while let Some(RawValue(value)) = seq.next_element()? {
            values.push(value);
        }
        Ok(RawValue(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error>
    {
        let mut fields = BTreeMap::new();
        while let Some((key, RawValue(value))) = map.next_entry()? {
            fields.insert(key, value);
        }
        Ok(RawValue(Value::Object(fields)))
    }
}

impl RawValue
{
    #[doc(hidden)]
    pub fn field(&self, name: &str) -> Option<&Value>
    {
        match &self.0 {
            Value::Object(fields) => fields.get(name),
            _ => None,
        }
    }
}
