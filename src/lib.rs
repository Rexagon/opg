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

    #[test]
    fn test_super() {
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

        println!(
            "{}",
            serde_yaml::to_string(&NewType::get_structure()).unwrap()
        );
        println!("{}", serde_yaml::to_string(&Test::get_structure()).unwrap());
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
