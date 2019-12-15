//! Support for cbor tags
use serde::ser::{Serialize, Serializer};

/// signals that a newtype is from a CBOR tag
pub(crate) const CBOR_NEWTYPE_NAME: &str = "\0cbor_tag";

/// A value that is optionally tagged with a cbor tag
///
/// this only serves as an intermediate helper for tag serialization or deserialization
pub struct Tagged<T> {
    /// cbor tag
    pub tag: Option<u64>,
    /// value
    pub value: T,
}

impl<T> Tagged<T> {
    /// Create a new tagged value
    pub fn new(tag: Option<u64>, value: T) -> Self {
        Self { tag, value }
    }

    /// Get the inner value if the cbor tag has the expected value
    pub fn unwrap_if_tag<'de, D: serde::de::Deserializer<'de>>(
        self,
        expected_tag: u64,
    ) -> Result<T, D::Error> {
        match self.tag {
            Some(tag) if tag == expected_tag => Ok(self.value),
            Some(_) => Err(serde::de::Error::custom("unexpected cbor tag")),
            None => Err(serde::de::Error::custom("missing cbor tag")),
        }
    }
}

impl<T: Serialize> Serialize for Tagged<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        set_tag(self.tag);
        let r = s.serialize_newtype_struct(CBOR_NEWTYPE_NAME, &self.value);
        set_tag(None);
        r
    }
}

fn untagged<T>(value: T) -> Tagged<T> {
    Tagged::new(None, value)
}

macro_rules! delegate {
    ($name: ident, $type: ty) => {
        fn $name<E: serde::de::Error>(self, v: $type) -> Result<Self::Value, E>
        {
            T::deserialize(v.into_deserializer()).map(untagged)
        }
    };
}

use serde::de::IntoDeserializer;

impl<'de, T: serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for Tagged<T> {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: serde::de::Deserialize<'de>> serde::de::Visitor<'de> for ValueVisitor<T> {
            type Value = Tagged<T>;

            fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.write_str("a cbor tag newtype")
            }

            delegate!(visit_bool, bool);
            
            delegate!(visit_i8, i8);
            delegate!(visit_i16, i16);
            delegate!(visit_i32, i32);
            delegate!(visit_i64, i64);

            delegate!(visit_u8, u8);
            delegate!(visit_u16, u16);
            delegate!(visit_u32, u32);
            delegate!(visit_u64, u64);
            
            delegate!(visit_f32, f32);
            delegate!(visit_f64, f64);
            
            delegate!(visit_char, char);
            delegate!(visit_str, &str);
            delegate!(visit_borrowed_str, &'de str);
            delegate!(visit_string, String);

            // delegate!(visit_bytes, &[u8]);
            // delegate!(visit_borrowed_bytes, &'de [u8]);
            delegate!(visit_byte_buf, Vec<u8>);

            fn visit_borrowed_bytes<E: serde::de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E>
            {
                T::deserialize(serde::de::value::BorrowedBytesDeserializer::new(value)).map(untagged)
            }

            // fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E>
            // {
            //     serde::de::value::NeverDeserializer
            //     T::deserialize(None.into_deserializer()).map(untagged)
            // }

            fn visit_some<D: serde::de::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error>
            {
                T::deserialize(deserializer).map(untagged)
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E>
            {
                T::deserialize(().into_deserializer()).map(untagged)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                T::deserialize(serde::de::value::SeqAccessDeserializer::new(seq)).map(untagged)
            }

            fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(untagged)
            }

            // fn visit_enum<A: serde::de::EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error>
            // {

            // }

            fn visit_newtype_struct<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error>
            {
                let t = get_tag();
                T::deserialize(deserializer).map(|v| Tagged::new(t, v))
            }
        }

        deserializer.deserialize_any(ValueVisitor::<T>(std::marker::PhantomData))
    }
}

#[cfg(feature = "tags")]
pub(crate) fn set_tag(value: Option<u64>) {
    CBOR_TAG.with(|f| *f.borrow_mut() = value);
}

#[cfg(feature = "tags")]
pub(crate) fn get_tag() -> Option<u64> {
    CBOR_TAG.with(|f| *f.borrow())
}

#[cfg(not(feature = "tags"))]
pub(crate) fn set_tag(_value: Option<u64>) {}

#[cfg(not(feature = "tags"))]
pub(crate) fn get_tag() -> Option<u64> {
    None
}

#[cfg(feature = "tags")]
use std::cell::RefCell;

#[cfg(feature = "tags")]
thread_local!(static CBOR_TAG: RefCell<Option<u64>> = RefCell::new(None));
