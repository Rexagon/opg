mod opg;

pub use opg::*;
pub use opg_proc::*;

use serde::Serialize;

#[cfg(feature = "test_compilation")]
mod test_compilation {
    use super::*;

    fn test_string() -> String {
        "AAA".to_owned()
    }

    #[derive(Serialize, OpgModel)]
    #[opg(with = "test_string")]
    #[serde(rename_all = "camelCase")]
    struct TempTest {
        asd: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super() {
        #[derive(Serialize, Opg)]
        struct Test {
            asd: u32,
        }

        assert_eq!(Test::example(), None);
    }

    #[test]
    fn test_with() {
        fn create_string() -> String {
            "Hello World".to_owned()
        }

        #[derive(Serialize, Opg)]
        #[opg(with = "create_string")]
        struct Test {
            asd: u32,
        }

        assert_eq!(Test::example(), Some("Hello World".to_owned()));
    }
}
