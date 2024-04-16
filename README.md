### serde_yaml2

This crate provides serde integration for [serde_yaml2](https://github.com/Ethiraric/yaml-rust2/)


##### Usage

```rust
use serde::{Deserialize, Serialize};
use serde_yaml2::{from_str, to_string};

#[derive(Deserialize, Debug, PartialEq)]
enum TestEnum {
    VariantA,
    VariantB(),
    VariantC(String),
    VariantD { x: f64, y: f64 },
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct TestStruct {
    x: i64,
    y: String,
    z: Vec<i32>,
}

// Deserialize from str
let result: TestStruct = from_str("x: -41\ny: Hello world\nz: [1,2,3]\n").unwrap();

assert_eq!(TestStruct {
    x: -41,
    y: "Hello world".to_owned(),
    z: vec![1,2,3],
}, result);

// Serialize back into String
let serialized = to_string(result).unwrap();
assert_eq!("'x':\n  -41\n'y':\n  'Hello world'\n'z':\n  - \n   1\n  - \n   2\n  - \n   3\n  \n", serialized);
```
