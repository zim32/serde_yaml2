use std::fmt::{Display, Formatter};
use std::str::Chars;
use serde::de::{Visitor, Error, SeqAccess, DeserializeSeed, MapAccess, EnumAccess, VariantAccess};
use serde::{Deserialize, Deserializer};
use yaml_rust2::{Event, Yaml};
use yaml_rust2::parser::{Parser};
use thiserror::Error;
use yaml_rust2::scanner::{Marker, TScalarStyle};

pub fn from_str<'de, T: Deserialize<'de>>(data: &'de str) -> Result<T, serde::de::value::Error> {
    let deserializer = &mut YamlDeserializer::from_str(data)?;
    T::deserialize(deserializer)
}

#[derive(Debug)]
struct MarkerWrapper(Marker);

impl Display for MarkerWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line: {}, Column: {}, Index: {}", self.0.line(), self.0.col(), self.0.index())
    }
}

#[derive(Error, Debug)]
enum Errors<'a> {
    #[error("Unexpected scalar value at position {2}. Expected: {0}, got: {1}")]
    UnexpectedScalarValueError(&'a str, &'a str, MarkerWrapper ),
    #[error("Unexpected event at position {2}. Expected: {0}, got: {1:?}")]
    UnexpectedEventError(&'a str, Event, MarkerWrapper),
    #[error("Scan error at position {0}")]
    ScanError(MarkerWrapper),
    #[error("Error while parsing scalar {0} into number")]
    ParseNumberError(&'a str),
}

impl<'a> Errors<'a> {
    fn unexpected_scalar_value_error(expected: &'a str, got: &'a str, marker: Marker) -> Self {
        Errors::UnexpectedScalarValueError(expected, got, MarkerWrapper(marker))
    }

    fn unexpected_event_error(expected: &'a str, got: Event, marker: Marker) -> Self {
        Errors::UnexpectedEventError(expected, got, MarkerWrapper(marker))
    }

    fn scan_error(marker: Marker) -> Self {
        Errors::ScanError(MarkerWrapper(marker))
    }

    fn parse_number_error(value: &'a str) -> Self {
        Errors::ParseNumberError(value)
    }
}

impl<'a> Into<serde::de::value::Error> for Errors<'a> {
    fn into(self) -> serde::de::value::Error {
        serde::de::value::Error::custom(self.to_string())
    }
}

struct EventsSequenceAccess<'a, 'de> {
    deserializer: &'a mut YamlDeserializer<'de>,
}

impl<'de, 'a> SeqAccess<'de> for EventsSequenceAccess<'a, 'de> {
    type Error = serde::de::value::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        if let Ok((Event::SequenceEnd, ..), ..) = self.deserializer.parser.peek() {
            return Ok(None);
        }

        seed.deserialize(&mut *self.deserializer).map(Some)
    }
}

impl<'de, 'a> MapAccess<'de> for EventsSequenceAccess<'a, 'de> {
    type Error = serde::de::value::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: DeserializeSeed<'de> {
        if let Ok((Event::MappingEnd, ..), ..) = self.deserializer.parser.peek() {
            return Ok(None);
        }

        seed.deserialize(&mut *self.deserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: DeserializeSeed<'de> {
        seed.deserialize(&mut *self.deserializer)
    }
}

impl<'de, 'a> EnumAccess<'de> for EventsSequenceAccess<'a, 'de> {
    type Error = serde::de::value::Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> where V: DeserializeSeed<'de> {
        let value = seed.deserialize(&mut *self.deserializer)?;
        Ok((value, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for EventsSequenceAccess<'a, 'de> {
    type Error = serde::de::value::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.deserializer.parser.next_token() {
            Ok((Event::Scalar(value, _, ..), marker), ..) => {
                if value == "null" || value == "~" {
                    Ok(())
                } else {
                    Err(Errors::unexpected_scalar_value_error("'~' or 'null'", &value, marker).into())
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error> where T: DeserializeSeed<'de> {
        seed.deserialize(&mut *self.deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserializer.deserialize_seq(visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserializer.deserialize_map(visitor)
    }
}


struct YamlValueAccess<'a, 'de, Y: Iterator<Item = Yaml>> {
    deserializer: &'a mut YamlDeserializer<'de>,
    yaml: Y,
}

impl<'de, 'a, Y: Iterator<Item = Yaml>> SeqAccess<'de> for YamlValueAccess<'a, 'de, Y> {
    type Error = serde::de::value::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        if let Some(_) = self.yaml.next() {
            seed.deserialize(&mut *self.deserializer).map(Some)
        } else {
            Ok(None)
        }
    }
}

macro_rules! deserialize_number {
    ($self:ident, $visitor:ident, $visit:ident, $type:ty) => {
        match $self.parser.next_token() {
            Ok((Event::Scalar(value, TScalarStyle::Plain, ..), ..), ..) => {
                let Ok(parsed) = value.parse::<$type>() else {
                    return Err(Errors::parse_number_error(&value).into());
                };
                return $visitor.$visit(parsed);
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }
}

pub struct YamlDeserializer<'de> {
    parser: Parser<Chars<'de>>,
}

impl<'de> YamlDeserializer<'de> {
    pub fn from_str(data: &'de str) -> Result<Self, serde::de::value::Error> {
        let mut parser = Parser::new_from_str(data);

        // skip stream and doc events
        if let Ok((Event::StreamStart, ..)) = parser.peek() {
            parser.next_token().map_err(|e| Errors::scan_error(*e.marker()).into())?;

            if let Ok((Event::DocumentStart, ..)) = parser.peek() {
                parser.next_token().map_err(|e| Errors::scan_error(*e.marker()).into())?;
            }
        }

        Ok(YamlDeserializer {
            parser
        })
    }
}

impl<'de, 'a> Deserializer<'de> for &'a mut YamlDeserializer<'de> {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((event, _)) => {
                match event {
                    Event::Scalar(value, _, _, _) => {
                        let yaml_node = Yaml::from_str(&value);

                        match yaml_node {
                            Yaml::Real(v) => {
                                let Ok(parsed) = v.parse() else {
                                    return Err(Errors::parse_number_error(&v).into());
                                };
                                visitor.visit_f64(parsed)
                            },
                            Yaml::Integer(v) => {
                                visitor.visit_i64(v)
                            },
                            Yaml::String(v) => {
                                visitor.visit_string(v)
                            },
                            Yaml::Boolean(v) => {
                                visitor.visit_bool(v)
                            },
                            Yaml::Null => {
                                visitor.visit_none()
                            },
                            _ => {
                                Err(Self::Error::custom("Unexpected Yaml node type"))
                            },
                        }
                    },
                    Event::SequenceStart(_, _) => {
                        let value = visitor.visit_seq(EventsSequenceAccess { deserializer: self })?;

                        if let Ok((Event::SequenceEnd, ..), ..) = self.parser.next_token() {
                            Ok(value)
                        } else {
                            Err(Self::Error::custom("Expected SequenceEnd event"))
                        }
                    },
                    Event::MappingStart(_, _) => {
                        let value = visitor.visit_map(EventsSequenceAccess { deserializer: self })?;

                        match self.parser.next_token() {
                            Ok((Event::MappingEnd, ..), ..) => {
                                Ok(value)
                            },
                            Ok((event, marker)) => {
                                Err(Errors::unexpected_event_error("MappingEnd", event, marker).into())
                            },
                            Err(scan_error) => {
                                Err(Errors::scan_error(*scan_error.marker()).into())
                            },
                        }
                    },
                    Event::SequenceEnd => {
                        Err(Self::Error::custom("Unexpected SequenceEnd event"))
                    },
                    Event::MappingEnd => {
                        Err(Self::Error::custom("Unexpected MappingEnd event"))
                    },
                    Event::Nothing => {
                        Err(Self::Error::custom("Unexpected Nothing event"))
                    }
                    Event::StreamStart => {
                        Err(Self::Error::custom("Unexpected StreamStart event"))
                    }
                    Event::StreamEnd => {
                        Err(Self::Error::custom("Unexpected StreamEnd event"))
                    }
                    Event::DocumentStart => {
                        Err(Self::Error::custom("Unexpected DocumentStart event"))
                    }
                    Event::DocumentEnd => {
                        Err(Self::Error::custom("Unexpected DocumentEnd event"))
                    }
                    Event::Alias(_) => {
                        Err(Self::Error::custom("Unexpected Alias event"))
                    }
                }
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::Scalar(value, TScalarStyle::Plain, ..), marker), ..) => {
                match value.as_str() {
                    "true" => {
                        return visitor.visit_bool(true);
                    },
                    "false" => {
                        return visitor.visit_bool(false);
                    },
                    _ => {
                        Err(Errors::unexpected_scalar_value_error("'true' of 'false'", &value, marker).into())
                    }
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Plain scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_i8, i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_i16, i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_i32, i32)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_i64, i64)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_u8, u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_u16, u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_u32, u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_u64, u64)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_f32, f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        deserialize_number!(self, visitor, visit_f64, f64)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::Scalar(value, TScalarStyle::Plain, ..), marker), ..) => {
                if value.len() != 1 {
                    Err(Errors::unexpected_scalar_value_error("String with length 1", &value, marker).into())
                } else {
                    let Some(char) = value.chars().next() else {
                        return Err(Errors::unexpected_scalar_value_error("String with single char", &value, marker).into());
                    };
                    visitor.visit_char(char)
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Plain scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(Self::Error::custom("Deserialization of &str is not supported"))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::Scalar(value, _, ..), ..), ..) => {
                return visitor.visit_string(value);
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(Self::Error::custom("Deserialization of bytes is not supported"))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(Self::Error::custom("Deserialization of byte buffer is not supported"))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.peek() {
            Ok((Event::Scalar(value, _, ..), ..), ..) => {
                if value == "null" || value == "~" {
                    self.parser.next_token().map_err(|e| Errors::scan_error(*e.marker()).into())?;
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Scalar", event.clone(), marker.clone()).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::Scalar(value, _, ..), marker), ..) => {
                if value == "null" || value == "~" {
                    visitor.visit_unit()
                } else {
                    Err(Errors::unexpected_scalar_value_error("'null' or '~'", &value, marker).into())
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("Scalar", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::SequenceStart(..), ..), ..) => {
                let value = visitor.visit_seq(EventsSequenceAccess { deserializer: self })?;

                if let Ok((Event::SequenceEnd, ..), ..) = self.parser.next_token() {
                    Ok(value)
                } else {
                    Err(Self::Error::custom("Expected SequenceEnd"))
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("SequenceStart", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_tuple<V>(self, _visitor: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::MappingStart(..), ..), ..) => {
                let value = visitor.visit_map(EventsSequenceAccess { deserializer: self })?;

                match self.parser.next_token() {
                    Ok((Event::MappingEnd, ..), ..) => {
                        Ok(value)
                    },
                    Ok((event, marker)) => {
                        Err(Errors::unexpected_event_error("MappingEnd", event, marker).into())
                    },
                    Err(scan_error) => {
                        Err(Errors::scan_error(*scan_error.marker()).into())
                    },
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("MappingStart", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.parser.next_token() {
            Ok((Event::MappingStart(..), ..), ..) => {
                let value = visitor.visit_enum(EventsSequenceAccess { deserializer: self })?;

                if let Ok((Event::MappingEnd, ..), ..) = self.parser.next_token() {
                    Ok(value)
                } else {
                    Err(Self::Error::custom("Expected MappingEnd"))
                }
            },
            Ok((event, marker)) => {
                Err(Errors::unexpected_event_error("MappingStart", event, marker).into())
            },
            Err(scan_error) => {
                Err(Errors::scan_error(*scan_error.marker()).into())
            },
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_any(visitor)
    }
}


#[cfg(test)]
mod tests {
    use crate::de::YamlDeserializer;
    use serde::{Deserialize};
    use yaml_rust2::Yaml;
    use crate::wrapper::YamlNodeWrapper;

    macro_rules! test {
        ($type:ty, $expected:expr, $data:literal) => {
            {
                let deserializer = &mut YamlDeserializer::from_str($data).unwrap();
                let result: $type = <$type as Deserialize>::deserialize(deserializer).unwrap();
                assert_eq!($expected, result);
            }
        }
    }

    #[test]
    fn should_work() {
        test!((), (), "null");
        test!((), (), "~");

        test!(char, 'a', "a");

        test!(i8,  123, "123");
        test!(i16, 123, "123");
        test!(i32, 123, "123");
        test!(i64, 123, "123");
        test!(u8,  123, "123");
        test!(u16, 123, "123");
        test!(u32, 123, "123");
        test!(u64, 123, "123");
        test!(i64, -123, "-123");

        test!(f64, 123.341, "123.341");
        test!(f64, -123.341 ,"-123.341");

        test!(f32, 123.341, "123.341");
        test!(f32, -123.341, "-123.341");

        test!(f64, 0.0, "0");
        test!(f64, 0.0, "-0");

        test!(bool, true, "true");
        test!(bool, false, "false");

        test!(String, "Hello double quotes", r#""Hello double quotes""#);
        test!(String, "Hello single quotes", r#"'Hello single quotes'"#);
        test!(String, "Hello no quotes", r#"Hello no quotes"#);


        test!(Option<i32>, Some(32), "32");
        test!(Option<i32>, None, "~");
        test!(Option<i32>, None, "null");

        test!(String, "This is multiline string\n", r#"
>
    This is
    multiline
    string
"#);

        test!(String, "This is\nmultiline\nstring\n", r#"
|
    This is
    multiline
    string
"#);

        test!(String, "This is multiline string", r#"
>-
    This is
    multiline
    string
"#);

        test!(String, "This is\nmultiline\nstring", r#"
|-
    This is
    multiline
    string
"#);

        test!(Vec<i32>, Vec::<i32>::from([1,2,3]), "[1,2,3]");
        test!(Vec<Option<i32>>, Vec::<Option<i32>>::from([Some(1), Some(2), Some(3)]), "[1,2,3]");
        test!(Vec<Option<i32>>, Vec::<Option<i32>>::from([Some(1), None, Some(3)]), "[1,~,3]");
        test!(Vec<i32>, Vec::<i32>::new(), "[]");

        test!(Vec<i32>, Vec::<i32>::from([1,2,3]), r#"
- 1
- 2
- 3
"#);

        test!((i32, i8, i64), (1,2,3), "[1,2,3]");
        test!((), (), "~");
        test!((), (), "null");
        test!((i32, i8, i64), (321, 12, -4123), r#"
- 321
- 12
- -4123
"#);

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestUnitStruct;
        test!(TestUnitStruct, TestUnitStruct, "null");
        test!(TestUnitStruct, TestUnitStruct, "~");

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestEmptyTupleStruct();
        test!(TestEmptyTupleStruct, TestEmptyTupleStruct(), "[]");

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestTupleStruct(i32, String, bool, f64);
        test!(TestTupleStruct, TestTupleStruct(32, String::from("Hello string"), true, 45.0), r#"[32, "Hello string", true, 45.0]"#);

        test!(TestTupleStruct, TestTupleStruct(123, String::from("Hello world"), true, 123.0), r#"
- 123
- Hello world
- true
- 123.00
"#);

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestStruct {
            x: i32,
            y: String,
        }
        test!(TestStruct, TestStruct { x: 3123, y: String::from("Hello world") },  r#"
x: 3123
y: Hello world
"#);

        #[derive(Deserialize, Debug, PartialEq)]
        enum TestEnum {
            VariantA,
            VariantB(),
            VariantC(i32, String),
            VariantD(TestStruct),
        }
        test!(TestEnum, TestEnum::VariantA,  r#"VariantA: ~"#);
        test!(TestEnum, TestEnum::VariantB(), r#"VariantB: []"#);
        test!(TestEnum, TestEnum::VariantC(12, String::from("Hello world")), r#"VariantC: [12, 'Hello world']"#);
        test!(TestEnum, TestEnum::VariantD(TestStruct { x: 12, y: String::from("Hello world") }), r#"
VariantD:
    x: 12
    y: Hello world
"#);

        {
            type Map = std::collections::HashMap<String, String>;
            test!(Map, Map::from([(String::from("foo"), String::from("321"))]), r#"foo: 321"#);
            test!(Map, Map::from([(String::from("foo"), String::from("321"))]), r#"foo: '321'"#);
            test!(Map, Map::from([(String::from("foo"), String::from("321"))]), r#"foo: "321""#);
        }

        {
            type Map = std::collections::HashMap<String, Option<String>>;
            test!(Map, Map::from([(String::from("foo"), None)]), r#"foo: ~"#);
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestStructWithWrapper {
            kind: String,
            data: YamlNodeWrapper
        }

        test!(TestStructWithWrapper, TestStructWithWrapper {
            kind: String::from("Test"),
            data: YamlNodeWrapper::new(Yaml::Array(vec![Yaml::String("Hello".to_owned()), Yaml::String("world".to_owned())])),
        }, "kind: Test\ndata: [ 'Hello', 'world' ]");
    }
}
