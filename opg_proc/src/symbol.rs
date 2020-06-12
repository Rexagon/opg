use std::fmt::{self, Display};
use syn::export::Formatter;
use syn::{Ident, Path};

macro_rules! define_symbols(
    ($($name:ident => $value:literal),*,) => {
        $(pub const $name: Symbol = Symbol($value));*;
    };
);

define_symbols! {
    // main macro name
    OPG => "opg",

    // named values
    EXAMPLE => "example",
    EXAMPLE_WITH => "example_with",
    FORMAT => "format",
    SUMMARY => "summary",
    DESCRIPTION => "description",

    // flags
    ENUM_STRING => "enum_string",
    STRING => "string",
    NUMBER => "number",
    INTEGER => "integer",
    BOOLEAN => "boolean",
    ARRAY => "array",
    OBJECT => "object",
    ONE_OF => "one_of",
    ANY => "any",

    INLINE => "inline",

    // serde
    SERDE => "serde",
    UNTAGGED => "untagged",
    TRANSPARENT => "transparent",
    FLATTEN => "flatten",
    SKIP => "skip",
    SKIP_SERIALIZING => "skip_serializing",
    SKIP_SERIALIZING_IF => "skip_serializing_if",
    TAG => "tag",
    CONTENT => "content",
    RENAME => "rename",
    RENAME_ALL => "rename_all",
    SERIALIZE => "serialize",
    DESERIALIZE => "deserialize",
}

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

impl Symbol {
    pub fn inner(&self) -> &'static str {
        self.0
    }
}

impl PartialEq<Symbol> for Ident {
    fn eq(&self, other: &Symbol) -> bool {
        self == other.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, other: &Symbol) -> bool {
        *self == other.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, other: &Symbol) -> bool {
        self.is_ident(other.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, other: &Symbol) -> bool {
        self.is_ident(other.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
