use serde::{Deserialize, Serialize};
use serde_yaml2::{from_str, to_string};

#[test]
fn deserialize() {
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

    let result: TestStruct = from_str("x: -41\ny: Hello world\nz: [1,2,3]\n").unwrap();

    assert_eq!(TestStruct {
        x: -41,
        y: "Hello world".to_owned(),
        z: vec![1,2,3],
    }, result);

    let result = to_string(result).unwrap();
    assert_eq!("'x':\n  -41\n'y':\n  'Hello world'\n'z':\n  - \n   1\n  - \n   2\n  - \n   3\n  \n", result);

    assert_eq!(TestStruct {
        x: -41,
        y: "Hello world".to_owned(),
        z: vec![1,2,3],
    }, from_str(&result).unwrap());
}
