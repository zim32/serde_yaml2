use std::fmt::Formatter;
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq};
use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;
use serde::ser::Error as _;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct YamlNodeWrapper(Yaml);

impl YamlNodeWrapper {
    pub fn new(value: Yaml) -> Self {
        Self(value)
    }

    pub fn get(self) -> Yaml {
        self.0
    }
}

struct YamlNodeWrapperVisitor {}

impl<'de> Visitor<'de> for YamlNodeWrapperVisitor {
    type Value = Yaml;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("Yaml node")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Boolean(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Integer(v as i64))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Real(v.to_string()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Real(v.to_string()))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::String(v.into()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::String(v.into()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::String(v))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> where E: Error {
        Ok(Yaml::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let mut result: Vec<Yaml> = vec![];

        while let Ok(Some(el)) = seq.next_element::<YamlNodeWrapper>() {
            result.push(el.get())
        }

        Ok(Yaml::Array(result))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
        let mut result = Hash::new();

        while let Ok(Some((k,v))) = map.next_entry::<YamlNodeWrapper, YamlNodeWrapper>() {
            result.insert(k.get(), v.get());
        }

        Ok(Yaml::Hash(result))
    }
}

impl<'de> Deserialize<'de> for YamlNodeWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(YamlNodeWrapperVisitor {}).map(|i| YamlNodeWrapper(i))
    }
}

impl Serialize for YamlNodeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self.0 {
            Yaml::Real(v) => {
                let Ok(parsed) = v.parse() else {
                    return Err(S::Error::custom("Can not parse real number"));
                };
                serializer.serialize_f64(parsed)
            },
            Yaml::Integer(v) => {
                serializer.serialize_i64(*v)
            },
            Yaml::String(v) => {
                serializer.serialize_str(&v)
            },
            Yaml::Boolean(v) => {
                serializer.serialize_bool(*v)
            },
            Yaml::Array(v) => {
                let mut seq = serializer.serialize_seq(Some(v.len()))?;

                for el in v {
                    seq.serialize_element(&YamlNodeWrapper(el.clone()))?;
                }

                seq.end()
            },
            Yaml::Hash(v) => {
                let mut map = serializer.serialize_map(Some(v.len()))?;

                for (k, v) in v {
                    map.serialize_key(&YamlNodeWrapper(k.clone()))?;
                    map.serialize_value(&YamlNodeWrapper(v.clone()))?;
                }

                map.end()
            },
            Yaml::Null => {
                serializer.serialize_unit()
            },
            Yaml::Alias(_) | Yaml::BadValue => {
                unimplemented!()
            },
        }
    }
}
