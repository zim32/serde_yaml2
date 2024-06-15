use std::fmt::{Display, Write};
use serde::{Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant};

macro_rules! serialize_value {
    ($self:ident, $value:ident) => {
        write!($self.writer, "{}", $value)?;
        return Ok(());
    }
}

pub fn to_string<T: Serialize>(value: T) -> Result<String, Errors> {
    let mut out = String::new();
    let mut serializer = YamlSerializer::new(&mut out);
    serializer.write(value)?;
    Ok(out)
}

#[derive(thiserror::Error, Debug)]
pub enum Errors {
    #[error("Format arguments error")]
    FormArgsError,
    #[error("{0}")]
    UnsupportedSerializationError(String),
}

impl serde::ser::Error for Errors {
    fn custom<T>(msg: T) -> Self where T: Display {
        Self::UnsupportedSerializationError(msg.to_string())
    }
}

impl From<std::fmt::Error> for Errors {
    fn from(_value: std::fmt::Error) -> Self {
        Errors::FormArgsError
    }
}

fn write_indent(level: i32, writer: &mut dyn Write) -> Result<(), Errors> {
    if level <= 0 {
        return Ok(());
    }

    for _ in 0..=level {
        write!(writer, " ")?;
    }

    Ok(())
}

fn escape_str(source: &str) -> String {
    source.replace('\'', r#"\'"#)
}

pub struct SequenceSerializer<'a, 'se> {
    len: Option<usize>,
    ser: &'a mut YamlSerializer<'se>,
}

impl<'a, 'se> SequenceSerializer<'a, 'se> {
    fn process_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Errors> {
        self.ser.writer.write_str("- \n")?;
        self.ser.incr_level();
        write_indent(self.ser.level, self.ser.writer)?;
        T::serialize(value, &mut *self.ser)?;
        self.ser.decr_level();
        self.ser.writer.write_char('\n')?;
        write_indent(self.ser.level, self.ser.writer)
    }

    fn process_end(self) -> Result<(), Errors> {
        if let Some(0) = self.len {
            self.ser.writer.write_char(']')?;
        }

        Ok(())
    }
}

impl<'a, 'se> SerializeSeq for SequenceSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.process_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.process_end()
    }
}

impl<'a, 'se> SerializeTuple for SequenceSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.process_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.process_end()
    }
}

impl<'a, 'se> SerializeTupleStruct for SequenceSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.process_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.process_end()
    }
}

impl<'a, 'se> SerializeTupleVariant for SequenceSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.process_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {

        if let Some(l) = self.len {
            if l > 0 {
                self.ser.decr_level();
            }
        }

        self.process_end()
    }
}

pub struct MapSerializer<'a, 'se> {
    ser: &'a mut YamlSerializer<'se>,
}

impl<'a, 'se> MapSerializer<'a, 'se> {
    fn process_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Errors> {
        T::serialize(key, &mut *self.ser)?;
        self.ser.writer.write_str(":\n")?;
        self.ser.incr_level();
        write_indent(self.ser.level, self.ser.writer)
    }

    fn process_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Errors> {
        T::serialize(value, &mut *self.ser)?;
        self.ser.decr_level();
        self.ser.writer.write_char('\n')?;
        write_indent(self.ser.level, self.ser.writer)
    }

    fn process_end(self) -> Result<(), Errors> {
        Ok(())
    }
}

impl<'a, 'se> SerializeMap for MapSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.process_key(key)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.process_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.process_end()
    }
}

impl<'a, 'se> SerializeStruct for MapSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        self.process_key(key)?;
        self.process_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.process_end()
    }
}

impl<'a, 'se> SerializeStructVariant for MapSerializer<'a, 'se> {
    type Ok = ();
    type Error = Errors;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        self.process_key(key)?;
        self.process_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.decr_level();
        self.process_end()
    }
}

trait TypeWriter<'se> {
    type Ok;
    type Error;

    fn write_str(v: &str, level: i32, writer: &'se mut dyn Write) -> Result<Self::Ok, Self::Error>;
}

struct InHouseTypeWriter {}

impl<'se> TypeWriter<'se> for InHouseTypeWriter {
    type Ok = ();
    type Error = Errors;

    fn write_str(v: &str, level: i32, writer: &'se mut dyn Write) -> Result<Self::Ok, Self::Error> {
        if v.contains('\n') {
            write!(writer, "|-")?;

            for line in v.lines() {
                writeln!(writer)?;
                write_indent(level + 1, writer)?;
                write!(writer, "{line}")?;
            }

            Ok(())
        } else {
            write!(writer, "'{}'", escape_str(v))?;
            Ok(())
        }
    }
}

pub struct YamlSerializer<'se> {
    level: i32,
    writer: &'se mut dyn Write,
}

impl<'se> YamlSerializer<'se> {
    pub fn new(writer: &'se mut dyn std::fmt::Write) -> Self {
        YamlSerializer {
            level: 0,
            writer
        }
    }

    pub fn write<T: Serialize>(&'se mut self, data: T) -> Result<(), Errors> {
        data.serialize(self)
    }

    fn incr_level(&mut self) -> i32 {
        self.level += 1;
        self.level
    }

    fn decr_level(&mut self) -> i32 {
        assert_ne!(self.level, -1);
        self.level -= 1;
        self.level
    }
}

impl<'a, 'se> Serializer for &'a mut YamlSerializer<'se> {
    type Ok = ();
    type Error = Errors;
    type SerializeSeq = SequenceSerializer<'a, 'se>;
    type SerializeTuple = SequenceSerializer<'a, 'se>;
    type SerializeTupleStruct = SequenceSerializer<'a, 'se>;
    type SerializeTupleVariant = SequenceSerializer<'a, 'se>;
    type SerializeMap = MapSerializer<'a, 'se>;
    type SerializeStruct = MapSerializer<'a, 'se>;
    type SerializeStructVariant = MapSerializer<'a, 'se>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match v {
            true => self.writer.write_str("true")?,
            false => self.writer.write_str("false")?,
        };

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        serialize_value!(self, v);
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        InHouseTypeWriter::write_str(v, self.level, self.writer)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Errors::UnsupportedSerializationError("Serialization of bytes is not supported".to_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_char('~')?;
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        write!(self.writer, "{}: ", variant)?;
        self.serialize_none()
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _name: &'static str, _variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error> {
        writeln!(self.writer, "{}:", variant)?;
        self.incr_level();
        write_indent(self.level, self.writer)?;
        let result = value.serialize(&mut *self);
        self.decr_level();
        result
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if let Some(0) = len {
            self.writer.write_char('[')?;
            Ok(SequenceSerializer {
                len: Some(0),
                ser: self,
            })
        } else {
            Ok(SequenceSerializer {
                len,
                ser: self,
            })
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        if len == 0 {
            write!(self.writer, "{}: ", variant)?;
            self.serialize_seq(Some(len))
        } else {
            writeln!(self.writer, "{}:", variant)?;
            self.incr_level();
            write_indent(self.level, self.writer)?;
            self.serialize_seq(Some(len))
        }
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            ser: self
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(self, name: &'static str, _variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        writeln!(self.writer, "{}:", variant)?;
        self.incr_level();
        write_indent(self.level, self.writer)?;
        self.serialize_struct(name, len)
    }
}

#[cfg(test)]
mod test {
    use crate::ser::YamlSerializer;
    use serde::{Serialize};

    macro_rules! test {
        ($data:expr, $expected:literal) => {
            let data = $data;
            let mut output = String::new();
            let mut serializer = YamlSerializer::new(&mut output);
            serializer.write(data).unwrap();
            assert_eq!($expected, output);
        }
    }

    #[test]
    fn should_work() {
        // primitives
        test!(123, "123");
        test!(0, "0");
        test!(0.0, "0");
        test!(0.53, "0.53");
        test!(100u8, "100");
        test!(100u16, "100");
        test!(100u32, "100");
        test!(100u64, "100");
        test!(-100i16, "-100");
        test!(-100i32, "-100");
        test!(-100i64, "-100");

        test!(0.54f32, "0.54");
        test!(0.54f64, "0.54");
        test!(-0.54f32, "-0.54");
        test!(-0.54f64, "-0.54");

        test!(true, "true");
        test!(false, "false");

        test!((), "~");
        test!('a', "a");
        test!("", "''");

        // strings
        test!("Hello world", "'Hello world'");
        test!(":Hello world", "':Hello world'");
        test!("Hello 'world", r#"'Hello \'world'"#);
        test!(String::from("Hello world"), "'Hello world'");
        test!("First\nSecond\nThird", "|-\n  First\n  Second\n  Third");
        test!("First\nSecond\nThird\n", "|-\n  First\n  Second\n  Third");


        // vectors
        test!(Vec::<i32>::from([1,2,3]), "- \n  1\n- \n  2\n- \n  3\n");
        test!(Vec::<i32>::new(), "[]");
        test!(Vec::<Vec<i32>>::from([[1,2,3].into(), [4,5,6].into(), [7,8,9].into()]), "- \n  - \n   1\n  - \n   2\n  - \n   3\n  \n- \n  - \n   4\n  - \n   5\n  - \n   6\n  \n- \n  - \n   7\n  - \n   8\n  - \n   9\n  \n");

        // tuples
        test!((123, "Hello world", false), "- \n  123\n- \n  'Hello world'\n- \n  false\n");
        test!((123, "Hello world", [1,2,3].to_vec()), "- \n  123\n- \n  'Hello world'\n- \n  - \n   1\n  - \n   2\n  - \n   3\n  \n");

        // structs
        #[derive(Serialize, Debug)]
        struct TestUnitStruct;
        test!(TestUnitStruct, "~");

        #[derive(Serialize, Debug)]
        struct TestEmptyTupleStruct();
        test!(TestEmptyTupleStruct(), "[]");

        #[derive(Serialize, Debug)]
        struct TestTupleStruct(i32, bool, String, Option<f64>);
        test!(TestTupleStruct(123, false, String::from("Hello world"), Some(4.5)), "- \n  123\n- \n  false\n- \n  'Hello world'\n- \n  4.5\n");
        test!(TestTupleStruct(123, false, String::from("Hello world"), None), "- \n  123\n- \n  false\n- \n  'Hello world'\n- \n  ~\n");

        #[derive(Serialize, Debug)]
        struct TestStruct {
            x: i32,
            y: String,
        }
        test!(TestStruct { x: 321, y: String::from("Hello") }, "'x':\n  321\n'y':\n  'Hello'\n");

        // enums
        #[derive(Serialize, Debug)]
        enum TestEnum {
            VariantA,
            VariantB(),
            VariantC(i32, String), // tuple variant
            VariantD(TestStruct), // new-type variant
            VariantE { x: f64, y: bool }, // struct variant
        }
        test!(TestEnum::VariantA, "VariantA: ~");
        test!(TestEnum::VariantB(), "VariantB: []");
        test!(TestEnum::VariantC(3000, String::from("Hello world")), "VariantC:\n  - \n   3000\n  - \n   'Hello world'\n  ");
        test!(TestEnum::VariantD(TestStruct { x: 1, y: String::from("Hello world") }), "VariantD:\n  'x':\n   1\n  'y':\n   'Hello world'\n  ");
        test!(TestEnum::VariantE { x: 45.0, y: false }, "VariantE:\n  'x':\n   45\n  'y':\n   false\n  ");

        // nested struct
        #[derive(Serialize, Debug)]
        struct TestNestedStruct {
            x: i32,
            nested: TestStruct,
            y: TestEnum,
            z: Vec<i32>,
            i: bool,
            b: TestEnum,
            u: (i32, String, bool),
        }
        test!(TestNestedStruct {
            x: 123,
            nested: TestStruct { x: 321, y: String::from("Hello world") },
            y: TestEnum::VariantD(
                TestStruct { x: 444, y: String::from("Hello\nworld") }
            ),
            z: vec![1,2,3],
            i: true,
            b: TestEnum::VariantE { x: 12.321, y: true },
            u: (555, String::from("Hello world"), false),
        }, "'x':\n  123\n'nested':\n  'x':\n   321\n  'y':\n   'Hello world'\n  \n'y':\n  VariantD:\n   'x':\n    444\n   'y':\n    |-\n     Hello\n     world\n   \n'z':\n  - \n   1\n  - \n   2\n  - \n   3\n  \n'i':\n  true\n'b':\n  VariantE:\n   'x':\n    12.321\n   'y':\n    true\n   \n'u':\n  - \n   555\n  - \n   'Hello world'\n  - \n   false\n  \n");

        {
            type Map = std::collections::BTreeMap<String, i32>;
            test!(Map::from([ (String::from("Hello"), 123), (String::from("World"), 10) ]), "'Hello':\n  123\n'World':\n  10\n");
        }
    }
}
