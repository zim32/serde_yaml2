use serde::{Serialize};
use serde_yaml2::{to_string};

fn main() {
    #[derive(Serialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
        z: i32,
    }

    #[derive(Serialize, Debug, PartialEq)]
    enum TestEnum {
        VariantA,
        VariantB(),
        VariantC(i32, f64),
        VariantD(Point),
        VariantE { a: bool, b: i32 },
    }

    let result = to_string(TestEnum::VariantA).unwrap();
    assert_eq!("VariantA: ~", result);

    let result = to_string(TestEnum::VariantB()).unwrap();
    assert_eq!("VariantB: []", result);

    let result = to_string(TestEnum::VariantC(123, 45.0)).unwrap();
    assert_eq!("VariantC:\n  - \n   123\n  - \n   45\n  ", result);

    let result = to_string(TestEnum::VariantD(Point { x: 1, y: 2, z: 3 })).unwrap();
    assert_eq!("VariantD:\n  'x':\n   1\n  'y':\n   2\n  'z':\n   3\n  ", result);

    let result = to_string(TestEnum::VariantE{ a: true, b: 3 }).unwrap();
    assert_eq!("VariantE:\n  'a':\n   true\n  'b':\n   3\n  ", result);
}
