pub trait FromStrangeTuple<T> {
    fn extract(self) -> Option<T>;
}

impl<T> FromStrangeTuple<T> for () {
    fn extract(self) -> Option<T> {
        None
    }
}

impl<T> FromStrangeTuple<T> for (T,) {
    fn extract(self) -> Option<T> {
        Some(self.0)
    }
}

#[macro_export]
macro_rules! describe_type(
    (raw_model => $model:ident) => {
        $model
    };

    (raw_type => {
        $(description: $description:literal)?
        ident: $type:ident
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $type,
        }
    };

    (string => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
        $(variants: [$($variants:literal),*])?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::String($crate::ModelString {
                variants: $crate::macros::FromStrangeTuple::extract(($(vec![$($variants.to_string()),*],)?)),
                data: ModelSimple {
                    format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                    example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
                }
            }))
        }
    };

    (number => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::Number($crate::ModelSimple {
                format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
            }))
        }
    };

    (integer => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::Integer($crate::ModelSimple {
                format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
            }))
        }
    };

    (boolean => {
        $(description: $description:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::Boolean)
        }
    };

    (array => {
        $(description: $description:literal)?
        items: ($($property_tail:tt)*)
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::Array($crate::ModelArray {
                items: Box::new(describe_type!(@object_property_value $($property_tail)*))
            }))
        }
    };

    (object => {
        $(description: $description:literal)?
        properties: {
            $($property_name:ident$([$required:tt])?: ($($property_tail:tt)*))*
        }
    }) => {{
        let mut properties = std::collections::BTreeMap::new();
        #[allow(unused_mut)]
        let mut required = Vec::new();

        $(describe_type!(@object_property [properties, required] $property_name$([$required])?: ($($property_tail)*)));*;

        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelTypeDescription::Object($crate::ModelObject {
                properties,
                required,
                ..Default::default()
            }))
        }
    }};

    (@object_property [$properties:ident, $required:ident] $property_name:ident: ($($property_tail:tt)*)) => {
        $properties.insert(stringify!($property_name).to_string(), describe_type!(@object_property_value $($property_tail)*));
    };


    (@object_property [$properties:ident, $required:ident] $property_name:ident[required]: ($($property_tail:tt)*)) => {
        describe_type!(@object_property [$properties, $required] $property_name: ($($property_tail)*));
        $required.push(stringify!($property_name).to_owned());
    };

    (@object_property_value link => $ref:literal) => {
        $crate::ModelReference::Link($ref.to_owned())
    };

    (@object_property_value link => $ref:ident) => {
        $crate::ModelReference::Link($ref.to_owned())
    };

    (@object_property_value $type:ident => $($tail:tt)*) => {
        $crate::ModelReference::Inline(describe_type!($type => $($tail)*))
    }
);

#[macro_export]
macro_rules! impl_opg_model(
    ($type:ty => $serialized_type:ident) => {
        impl $crate::OpgModel for $type {
            fn get_structure() -> Model {
                describe_type!($serialized_type => {})
            }
        }
    };

    ($type:ty => $serialized_type:ident always_inline) => {
        impl $crate::OpgModel for $type {
            fn get_structure() -> Model {
                describe_type!($serialized_type => {})
            }

            fn select_reference(_: bool, inline_params: &ContextParams, _: &str) -> ModelReference {
                Self::inject(InjectReference::Inline(inline_params))
            }
        }
    };
);
