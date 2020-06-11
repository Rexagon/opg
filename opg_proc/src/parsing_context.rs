use quote::ToTokens;
use std::cell::RefCell;

#[derive(Default)]
pub struct ParsingContext {
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl ParsingContext {
    pub fn new() -> Self {
        Self {
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    pub fn error_spanned_by<O, T>(&self, object: O, message: T)
    where
        O: ToTokens,
        T: std::fmt::Display,
    {
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            .push(syn::Error::new_spanned(object.into_token_stream(), message))
    }

    pub fn syn_error(&self, err: syn::Error) {
        self.errors.borrow_mut().as_mut().unwrap().push(err)
    }

    pub fn check(self) -> Result<(), Vec<syn::Error>> {
        let errors = self.errors.borrow_mut().take().unwrap();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Drop for ParsingContext {
    fn drop(&mut self) {
        if !std::thread::panicking() && self.errors.borrow().is_some() {
            panic!("forgot to check for errors");
        }
    }
}
