mod opg;

pub use opg_proc::*;

use serde::Serialize;

pub trait Example: Serialize {
    fn example() -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super() {
        #[derive(Serialize, Example)]
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

        #[derive(Serialize, Example)]
        #[example(with = "create_string")]
        struct Test {
            asd: u32,
        }

        assert_eq!(Test::example(), Some("Hello World".to_owned()));
    }
}
