use serde::{Serialize};
use serde_yaml2::{to_string};

#[derive(Serialize, Debug, PartialEq)]
struct TestStruct {
    x: i64,
    y: String,
    z: Vec<i32>,
}

fn main() {
    let value = TestStruct {
        x: -41,
        y: "Hello world".to_owned(),
        z: vec![1,2,3],
    };

    let serialized = to_string(value).unwrap();
    assert_eq!("'x':\n  -41\n'y':\n  'Hello world'\n'z':\n  - \n   1\n  - \n   2\n  - \n   3\n  \n", serialized);
}
