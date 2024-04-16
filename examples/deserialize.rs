use serde::{Deserialize};
use serde_yaml2::{from_str};

#[derive(Deserialize, Debug, PartialEq)]
struct TestStruct {
    x: i64,
    y: String,
    z: Vec<i32>,
}

fn main() {
    let result: TestStruct = from_str("x: -41\ny: Hello world\nz: [1,2,3]\n").unwrap();

    assert_eq!(TestStruct {
        x: -41,
        y: "Hello world".to_owned(),
        z: vec![1,2,3],
    }, result);
}
