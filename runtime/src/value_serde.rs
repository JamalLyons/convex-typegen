//! Serde support for Convex's wire value without passing through JSON.
//!
//! `None` serializes as `Value::Null`; use `skip_serializing_if` on optional struct fields when
//! Convex expects the key to be absent. This keeps explicit null distinct from omission.

use std::collections::BTreeMap;
use std::fmt;

use convex::Value;
use serde::de::{self, DeserializeOwned, DeserializeSeed, Deserializer as _, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize};

/// Error returned by the direct Convex value encoder and decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error
{
    path: String,
    message: String,
}

impl Error
{
    pub(crate) fn expected_object() -> Self
    {
        Self::new("$", "expected generated function arguments to serialize as an object")
    }

    fn new(path: impl Into<String>, message: impl Into<String>) -> Self
    {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    fn at(&self, path: &str) -> Self
    {
        if self.path.is_empty() || self.path == "$" {
            Self::new(path, self.message.clone())
        } else {
            self.clone()
        }
    }

    /// Path to the value that could not be encoded or decoded.
    pub fn path(&self) -> &str
    {
        if self.path.is_empty() { "$" } else { &self.path }
    }
}

impl fmt::Display for Error
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}: {}", if self.path.is_empty() { "$" } else { &self.path }, self.message)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error
{
    fn custom<T: fmt::Display>(message: T) -> Self
    {
        Self::new("$", message.to_string())
    }
}

impl de::Error for Error
{
    fn custom<T: fmt::Display>(message: T) -> Self
    {
        Self::new("$", message.to_string())
    }
}

/// Serialize a Rust value directly into Convex's value representation.
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, Error>
{
    value.serialize(ValueSerializer { path: "$".into() })
}

/// Deserialize a Rust value directly from Convex's value representation.
pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, Error>
{
    T::deserialize(ValueDeserializer { value, path: "$".into() })
}

/// A byte string that uses Serde's `serialize_bytes` / `deserialize_byte_buf` contract.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

impl From<Vec<u8>> for Bytes
{
    fn from(value: Vec<u8>) -> Self
    {
        Self(value)
    }
}

impl From<Bytes> for Vec<u8>
{
    fn from(value: Bytes) -> Self
    {
        value.0
    }
}

impl Serialize for Bytes
{
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Bytes
{
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        struct BytesVisitor;
        impl<'de> Visitor<'de> for BytesVisitor
        {
            type Value = Bytes;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                f.write_str("a byte string")
            }
            fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Bytes, E>
            {
                Ok(Bytes(bytes.to_vec()))
            }
            fn visit_byte_buf<E: de::Error>(self, bytes: Vec<u8>) -> Result<Bytes, E>
            {
                Ok(Bytes(bytes))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Bytes, A::Error>
            {
                let mut bytes = Vec::new();
                while let Some(byte) = seq.next_element::<u8>()? {
                    bytes.push(byte);
                }
                Ok(Bytes(bytes))
            }
        }
        deserializer.deserialize_byte_buf(BytesVisitor)
    }
}

#[derive(Clone)]
struct ValueSerializer
{
    path: String,
}

fn child_path(path: &str, key: &str) -> String
{
    if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        format!("{path}.{key}")
    } else {
        format!("{path}[{key:?}]")
    }
}

impl ser::Serializer for ValueSerializer
{
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Value, Error>
    {
        Ok(Value::Boolean(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_i16(self, v: i16) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_i32(self, v: i32) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_i64(self, v: i64) -> Result<Value, Error>
    {
        Ok(Value::Int64(v))
    }
    fn serialize_i128(self, v: i128) -> Result<Value, Error>
    {
        i64::try_from(v)
            .map(Value::Int64)
            .map_err(|_| Error::new(self.path, format!("integer {v} is outside Convex int64 range")))
    }
    fn serialize_u8(self, v: u8) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_u16(self, v: u16) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_u32(self, v: u32) -> Result<Value, Error>
    {
        self.serialize_i64(v.into())
    }
    fn serialize_u64(self, v: u64) -> Result<Value, Error>
    {
        i64::try_from(v)
            .map(Value::Int64)
            .map_err(|_| Error::new(self.path, format!("integer {v} is outside Convex int64 range")))
    }
    fn serialize_u128(self, v: u128) -> Result<Value, Error>
    {
        i64::try_from(v)
            .map(Value::Int64)
            .map_err(|_| Error::new(self.path, format!("integer {v} is outside Convex int64 range")))
    }
    fn serialize_f32(self, v: f32) -> Result<Value, Error>
    {
        Ok(Value::Float64(v.into()))
    }
    fn serialize_f64(self, v: f64) -> Result<Value, Error>
    {
        Ok(Value::Float64(v))
    }
    fn serialize_char(self, v: char) -> Result<Value, Error>
    {
        Ok(Value::String(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Value, Error>
    {
        Ok(Value::String(v.to_owned()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error>
    {
        Ok(Value::Bytes(v.to_vec()))
    }
    fn serialize_none(self) -> Result<Value, Error>
    {
        Ok(Value::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Value, Error>
    {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value, Error>
    {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Error>
    {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<Value, Error>
    {
        Ok(Value::String(variant.into()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> Result<Value, Error>
    {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<Value, Error>
    {
        let value = v.serialize(ValueSerializer {
            path: child_path(&self.path, variant),
        })?;
        Ok(Value::Object(BTreeMap::from([(variant.into(), value)])))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<SeqSerializer, Error>
    {
        Ok(SeqSerializer {
            path: self.path,
            values: Vec::new(),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<SeqSerializer, Error>
    {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<SeqSerializer, Error>
    {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantSerializer, Error>
    {
        Ok(TupleVariantSerializer {
            variant: variant.into(),
            inner: self.serialize_seq(Some(len))?,
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<MapSerializer, Error>
    {
        Ok(MapSerializer {
            path: self.path,
            values: BTreeMap::new(),
            key: None,
        })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<MapSerializer, Error>
    {
        self.serialize_map(None)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<StructVariantSerializer, Error>
    {
        Ok(StructVariantSerializer {
            variant: variant.into(),
            inner: self.serialize_map(None)?,
        })
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<Value, Error>
    {
        Ok(Value::String(value.to_string()))
    }
    fn is_human_readable(&self) -> bool
    {
        false
    }
}

pub struct SeqSerializer
{
    path: String,
    values: Vec<Value>,
}
impl ser::SerializeSeq for SeqSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error>
    {
        self.values.push(value.serialize(ValueSerializer {
            path: format!("{}[{}]", self.path, self.values.len()),
        })?);
        Ok(())
    }
    fn end(self) -> Result<Value, Error>
    {
        Ok(Value::Array(self.values))
    }
}
impl ser::SerializeTuple for SeqSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error>
    {
        ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<Value, Error>
    {
        ser::SerializeSeq::end(self)
    }
}
impl ser::SerializeTupleStruct for SeqSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error>
    {
        ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<Value, Error>
    {
        ser::SerializeSeq::end(self)
    }
}

pub struct TupleVariantSerializer
{
    variant: String,
    inner: SeqSerializer,
}
impl ser::SerializeTupleVariant for TupleVariantSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error>
    {
        ser::SerializeSeq::serialize_element(&mut self.inner, v)
    }
    fn end(self) -> Result<Value, Error>
    {
        Ok(Value::Object(BTreeMap::from([(
            self.variant,
            Value::Array(self.inner.values),
        )])))
    }
}

pub struct MapSerializer
{
    path: String,
    values: BTreeMap<String, Value>,
    key: Option<String>,
}
impl ser::SerializeMap for MapSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error>
    {
        if self.key.is_some() {
            return Err(Error::new(&self.path, "map key serialized before the previous value"));
        }
        self.key = Some(key.serialize(KeySerializer { path: self.path.clone() })?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error>
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::new(&self.path, "map value serialized before its key"))?;
        let path = child_path(&self.path, &key);
        if self.values.contains_key(&key) {
            return Err(Error::new(path, "duplicate Convex object key"));
        }
        self.values.insert(key, value.serialize(ValueSerializer { path })?);
        Ok(())
    }
    fn end(self) -> Result<Value, Error>
    {
        if self.key.is_some() {
            return Err(Error::new(self.path, "map ended after a key without a value"));
        }
        Ok(Value::Object(self.values))
    }
}
impl ser::SerializeStruct for MapSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    {
        self.values.insert(
            key.into(),
            value.serialize(ValueSerializer {
                path: child_path(&self.path, key),
            })?,
        );
        Ok(())
    }
    fn end(self) -> Result<Value, Error>
    {
        Ok(Value::Object(self.values))
    }
}
pub struct StructVariantSerializer
{
    variant: String,
    inner: MapSerializer,
}
impl ser::SerializeStructVariant for StructVariantSerializer
{
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    {
        ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
    }
    fn end(self) -> Result<Value, Error>
    {
        Ok(Value::Object(BTreeMap::from([(
            self.variant,
            Value::Object(self.inner.values),
        )])))
    }
}

struct KeySerializer
{
    path: String,
}
impl ser::Serializer for KeySerializer
{
    type Ok = String;
    type Error = Error;
    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;
    fn serialize_str(self, v: &str) -> Result<String, Error>
    {
        Ok(v.into())
    }
    fn serialize_char(self, v: char) -> Result<String, Error>
    {
        Ok(v.to_string())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> Result<String, Error>
    {
        v.serialize(self)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, v: &'static str) -> Result<String, Error>
    {
        Ok(v.into())
    }
    fn serialize_i8(self, _: i8) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_i16(self, _: i16) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_i32(self, _: i32) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_i64(self, _: i64) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_i128(self, _: i128) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_u8(self, _: u8) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_u16(self, _: u16) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_u32(self, _: u32) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_u64(self, _: u64) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_u128(self, _: u128) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_bool(self, _: bool) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_f32(self, _: f32) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_f64(self, _: f64) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_none(self) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_some<T: Serialize + ?Sized>(self, _: &T) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_unit(self) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<String, Error>
    {
        self.bad()
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Error>
    {
        self.bad()
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Error>
    {
        self.bad()
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct, Error>
    {
        self.bad()
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Error>
    {
        self.bad()
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error>
    {
        self.bad()
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Error>
    {
        self.bad()
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Error>
    {
        self.bad()
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, _: &T) -> Result<String, Error>
    {
        self.bad()
    }
    fn is_human_readable(&self) -> bool
    {
        false
    }
}
impl KeySerializer
{
    fn bad<T>(&self) -> Result<T, Error>
    {
        Err(Error::new(self.path.clone(), "Convex object keys must serialize as strings"))
    }
}

struct ValueDeserializer
{
    value: Value,
    path: String,
}
impl<'de> de::Deserializer<'de> for ValueDeserializer
{
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Int64(v) => visitor.visit_i64(v),
            Value::Float64(v) => visitor.visit_f64(v),
            Value::Boolean(v) => visitor.visit_bool(v),
            Value::String(v) => visitor.visit_string(v),
            Value::Bytes(v) => visitor.visit_byte_buf(v),
            Value::Array(v) => visitor.visit_seq(ValueSeq {
                iter: v.into_iter().enumerate(),
                path: self.path,
            }),
            Value::Object(v) => visitor.visit_map(ValueMap {
                iter: v.into_iter(),
                pending: None,
                path: self.path,
            }),
        }
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        match self.value {
            Value::Float64(value) => visitor.visit_f64(value),
            _ => Err(Error::new(self.path, "expected a float64")),
        }
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        match self.value {
            Value::Bytes(value) => visitor.visit_byte_buf(value),
            _ => Err(Error::new(self.path, "expected bytes")),
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        if matches!(self.value, Value::Null) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Error>
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        match self.value {
            Value::Array(v) => visitor.visit_seq(ValueSeq {
                iter: v.into_iter().enumerate(),
                path: self.path,
            }),
            _ => Err(Error::new(self.path, "expected an array")),
        }
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    {
        let value = visitor.visit_seq(match self.value {
            Value::Array(v) if v.len() == len => ValueSeq {
                iter: v.into_iter().enumerate(),
                path: self.path.clone(),
            },
            Value::Array(v) => {
                return Err(Error::new(
                    self.path,
                    format!("expected tuple of length {len}, found {}", v.len()),
                ));
            }
            _ => return Err(Error::new(self.path, "expected an array")),
        })?;
        Ok(value)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _: &'static str, len: usize, visitor: V) -> Result<V::Value, Error>
    {
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        match self.value {
            Value::Object(v) => visitor.visit_map(ValueMap {
                iter: v.into_iter(),
                pending: None,
                path: self.path,
            }),
            _ => Err(Error::new(self.path, "expected an object")),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    {
        self.deserialize_map(visitor)
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        self.deserialize_string(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    {
        let (variant, payload) = match self.value {
            Value::String(v) => (v, None),
            Value::Object(mut o) if o.len() == 1 => {
                let Some((k, v)) = o.pop_first() else {
                    return Err(Error::new(self.path, "expected a single-entry enum object"));
                };
                (k, Some(v))
            }
            _ => return Err(Error::new(self.path, "expected enum tag string or single-entry object")),
        };
        if !variants.contains(&variant.as_str()) {
            return Err(Error::new(self.path, format!("unknown enum variant {variant:?}")));
        }
        visitor.visit_enum(ValueEnum {
            variant,
            payload,
            path: self.path,
        })
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>
    {
        visitor.visit_unit()
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 char str string bytes unit unit_struct }
    fn is_human_readable(&self) -> bool
    {
        false
    }
}

struct ValueSeq
{
    iter: std::iter::Enumerate<std::vec::IntoIter<Value>>,
    path: String,
}
impl<'de> SeqAccess<'de> for ValueSeq
{
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    {
        match self.iter.next() {
            Some((i, v)) => {
                let path = format!("{}[{i}]", self.path);
                seed.deserialize(ValueDeserializer {
                    value: v,
                    path: path.clone(),
                })
                .map(Some)
                .map_err(|e| e.at(&path))
            }
            None => Ok(None),
        }
    }
    fn size_hint(&self) -> Option<usize>
    {
        Some(self.iter.len())
    }
}
struct ValueMap
{
    iter: std::collections::btree_map::IntoIter<String, Value>,
    pending: Option<(String, Value)>,
    path: String,
}
impl<'de> MapAccess<'de> for ValueMap
{
    type Error = Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    {
        self.pending = self.iter.next();
        match &self.pending {
            Some((key, _)) => seed
                .deserialize(serde::de::value::StringDeserializer::<Error>::new(key.clone()))
                .map(Some)
                .map_err(|e| e.at(&child_path(&self.path, key))),
            None => Ok(None),
        }
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error>
    {
        let (key, value) = self
            .pending
            .take()
            .ok_or_else(|| Error::new(&self.path, "map value requested without a key"))?;
        let path = child_path(&self.path, &key);
        seed.deserialize(ValueDeserializer {
            value,
            path: path.clone(),
        })
        .map_err(|e| e.at(&path))
    }
    fn size_hint(&self) -> Option<usize>
    {
        Some(self.iter.len() + usize::from(self.pending.is_some()))
    }
}
struct ValueEnum
{
    variant: String,
    payload: Option<Value>,
    path: String,
}
impl<'de> de::EnumAccess<'de> for ValueEnum
{
    type Error = Error;
    type Variant = ValueVariant;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, ValueVariant), Error>
    {
        let variant = seed.deserialize(serde::de::value::StringDeserializer::<Error>::new(self.variant.clone()))?;
        Ok((
            variant,
            ValueVariant {
                payload: self.payload,
                path: child_path(&self.path, &self.variant),
            },
        ))
    }
}
struct ValueVariant
{
    payload: Option<Value>,
    path: String,
}
impl<'de> de::VariantAccess<'de> for ValueVariant
{
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error>
    {
        match self.payload {
            None | Some(Value::Null) => Ok(()),
            _ => Err(Error::new(self.path, "expected unit enum variant")),
        }
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error>
    {
        seed.deserialize(ValueDeserializer {
            value: self.payload.ok_or_else(|| Error::new(&self.path, "missing enum payload"))?,
            path: self.path,
        })
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    {
        ValueDeserializer {
            value: self.payload.ok_or_else(|| Error::new(&self.path, "missing enum payload"))?,
            path: self.path,
        }
        .deserialize_tuple(len, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Error>
    {
        ValueDeserializer {
            value: self.payload.ok_or_else(|| Error::new(&self.path, "missing enum payload"))?,
            path: self.path,
        }
        .deserialize_struct("", fields, visitor)
    }
}

#[cfg(test)]
mod tests
{
    use std::collections::BTreeMap;

    use convex::Value;
    use serde::{Deserialize, Serialize};

    use super::{Bytes, Error, from_value, to_value};

    #[test]
    fn preserves_scalar_wire_kinds_and_non_finite_floats()
    {
        assert!(matches!(to_value(&()).unwrap(), Value::Null));
        assert!(matches!(to_value(&true).unwrap(), Value::Boolean(true)));
        assert!(matches!(to_value(&"text").unwrap(), Value::String(v) if v == "text"));
        assert!(matches!(to_value(&i64::MIN).unwrap(), Value::Int64(i64::MIN)));
        assert!(matches!(to_value(&i64::MAX).unwrap(), Value::Int64(i64::MAX)));
        assert!(matches!(to_value(&1.0_f64).unwrap(), Value::Float64(1.0)));
        assert!(matches!(to_value(&f64::INFINITY).unwrap(), Value::Float64(v) if v == f64::INFINITY));
        assert!(matches!(to_value(&f64::NEG_INFINITY).unwrap(), Value::Float64(v) if v == f64::NEG_INFINITY));
        assert!(matches!(to_value(&f64::NAN).unwrap(), Value::Float64(v) if v.is_nan()));

        let Value::Float64(negative_zero) = to_value(&-0.0_f64).unwrap() else {
            panic!("expected float64");
        };
        assert_eq!(negative_zero.to_bits(), (-0.0_f64).to_bits());
        let decoded: f64 = from_value(Value::Float64(negative_zero)).unwrap();
        assert_eq!(decoded.to_bits(), (-0.0_f64).to_bits());

        assert_eq!(from_value::<()>(Value::Null).unwrap(), ());
        assert!(from_value::<bool>(Value::Boolean(true)).unwrap());
        assert_eq!(from_value::<String>(Value::String("text".into())).unwrap(), "text");
        assert_eq!(from_value::<i64>(Value::Int64(i64::MIN)).unwrap(), i64::MIN);
        assert_eq!(from_value::<i64>(Value::Int64(i64::MAX)).unwrap(), i64::MAX);
        assert_eq!(from_value::<f64>(Value::Float64(1.0)).unwrap(), 1.0);
        assert_eq!(from_value::<f64>(Value::Float64(f64::INFINITY)).unwrap(), f64::INFINITY);
        assert_eq!(
            from_value::<f64>(Value::Float64(f64::NEG_INFINITY)).unwrap(),
            f64::NEG_INFINITY
        );
        assert!(from_value::<f64>(Value::Float64(f64::NAN)).unwrap().is_nan());
    }

    #[test]
    fn roundtrips_externally_tagged_enum_shapes()
    {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Event
        {
            Ready,
            Count(i64),
            Pair(String, bool),
            Record
            {
                label: String,
            },
        }

        for event in [
            Event::Ready,
            Event::Count(i64::MIN),
            Event::Pair("item".into(), true),
            Event::Record { label: "created".into() },
        ] {
            let encoded = to_value(&event).unwrap();
            assert_eq!(from_value::<Event>(encoded).unwrap(), event);
        }
    }

    #[test]
    fn preserves_bytes_and_recursive_records_and_arrays()
    {
        let bytes = Bytes(vec![0, 1, 127, 255]);
        assert!(matches!(to_value(&bytes).unwrap(), Value::Bytes(v) if v == bytes.0));
        assert_eq!(from_value::<Bytes>(Value::Bytes(vec![4, 5])).unwrap(), Bytes(vec![4, 5]));

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Nested
        {
            rows: Vec<BTreeMap<String, i64>>,
        }

        let mut row = BTreeMap::new();
        row.insert("count".to_owned(), 3);
        let nested = Nested { rows: vec![row] };
        let encoded = to_value(&nested).unwrap();
        assert_eq!(from_value::<Nested>(encoded).unwrap(), nested);
    }

    #[test]
    fn keeps_omitted_fields_separate_from_explicit_null()
    {
        #[derive(Serialize)]
        struct Args
        {
            #[serde(skip_serializing_if = "Option::is_none")]
            omitted: Option<String>,
            explicit_null: Option<Option<String>>,
        }

        let Value::Object(map) = to_value(&Args {
            omitted: None,
            explicit_null: Some(None),
        })
        .unwrap() else {
            panic!("expected object");
        };
        assert!(!map.contains_key("omitted"));
        assert!(matches!(map.get("explicit_null"), Some(Value::Null)));
    }

    #[test]
    fn reports_overflow_and_invalid_map_keys_with_paths()
    {
        #[derive(Serialize)]
        struct TooLarge
        {
            nested: u128,
        }

        let overflow = to_value(&TooLarge {
            nested: u128::from(u64::MAX) + 1,
        })
        .unwrap_err();
        assert_eq!(overflow.path(), "$.nested");

        let invalid_key = to_value(&BTreeMap::from([(7_i32, "value")])).unwrap_err();
        assert_eq!(invalid_key.path(), "$");
    }

    #[test]
    fn deserialization_type_errors_include_nested_path()
    {
        #[allow(dead_code)]
        #[derive(Debug, Deserialize)]
        struct Expected
        {
            nested: i64,
        }

        let error = from_value::<Expected>(Value::Object(BTreeMap::from([(
            "nested".to_owned(),
            Value::String("wrong".into()),
        )])))
        .unwrap_err();
        assert_eq!(error.path(), "$.nested");
    }

    #[test]
    fn error_path_is_root_for_non_object_args()
    {
        assert_eq!(Error::expected_object().path(), "$");
    }
}
