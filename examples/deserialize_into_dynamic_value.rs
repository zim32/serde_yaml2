use serde::{Deserialize};
use yaml_rust2::Yaml;
use serde_yaml2::{from_str};
use serde_yaml2::wrapper::YamlNodeWrapper;

#[derive(Deserialize, Debug, PartialEq)]
struct TestStruct {
    kind: String,
    data: YamlNodeWrapper,
}

fn main() {
    let result: TestStruct = from_str("kind: Foo\ndata: [1,2,3]\n").unwrap();

    assert_eq!(TestStruct {
        kind: "Foo".to_owned(),
        data: YamlNodeWrapper::new(
            Yaml::Array(
                vec![Yaml::Integer(1), Yaml::Integer(2), Yaml::Integer(3)]
            ),
        ),
    }, result);
}
