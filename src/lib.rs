mod opg;

pub use opg::*;
pub use opg_proc::*;

use serde::Serialize;

#[cfg(feature = "test_compilation")]
mod test_compilation {
    use super::*;

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    struct TempTest {
        asd: u32,
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    #[opg("New type description", string)]
    struct NewType(String);

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    struct Test {
        #[opg("Some description", inline)]
        asd: u32,
        hello_camel_case: NewType,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    #[opg("New type description", string)]
    struct NewType(String);

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    struct SimpleStruct {
        #[opg("Some description", inline)]
        asd: u32,
        hello_camel_case: NewType,
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "kebab-case")]
    #[opg("New type description", string)]
    enum StringEnumTest {
        First,
        Second,
        HelloWorld,
    }

    #[derive(Serialize, OpgModel)]
    #[serde(untagged)]
    enum UntaggedEnumTest {
        First {
            value: NewType,
        },
        #[opg("Very simple variant")]
        Second {
            #[opg("Very simple struct", inline)]
            another: SimpleStruct,
        },
    }

    #[test]
    fn test_super() {
        println!(
            "{}",
            serde_yaml::to_string(&NewType::get_structure()).unwrap()
        );
        println!(
            "{}",
            serde_yaml::to_string(&SimpleStruct::get_structure()).unwrap()
        );
    }

    #[test]
    fn test_string_enum() {
        println!(
            "{}",
            serde_yaml::to_string(&StringEnumTest::get_structure()).unwrap()
        );
    }

    #[test]
    fn test_untagged_enum() {
        println!(
            "{}",
            serde_yaml::to_string(&UntaggedEnumTest::get_structure()).unwrap()
        );
    }

    #[test]
    fn test_with() {
        #[derive(Serialize, OpgModel)]
        struct Test {
            asd: u32,
        }

        println!("{:?}", Test::get_structure());
    }
}
