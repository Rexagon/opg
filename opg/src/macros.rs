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

            fn select_reference(_: bool, inline_params: &ContextParams) -> ModelReference {
                Self::inject(InjectReference::Inline(inline_params))
            }
        }
    };

    (generic_tuple ($($type:ident),+)) => {
        impl<$($type),+> $crate::OpgModel for ($($type),+)
        where
            $($type : $crate::OpgModel),*
        {
            fn get_structure() -> Model {
                let item_model = $crate::Model {
                    description: None,
                    data: $crate::ModelData::OneOf($crate::ModelOneOf {
                        one_of: vec![
                            $(<$type as $crate::OpgModel>::select_reference(
                                false,
                                &Default::default(),
                            )),*
                        ],
                    }),
                };

                describe_type!(array => {
                    items: (raw_model => item_model)
                })
            }
        }
    };
);

#[macro_export]
macro_rules! describe_api {
    ($($property:ident: {$($property_value:tt)+}),*$(,)?) => {{
        let mut components = $crate::models::OpgComponents::default();
        $crate::models::Opg {
            $($property: { describe_api!(@opg_property components $property $($property_value)*) },)*
            components,
            ..Default::default()
        }
    }};


    (@opg_property $components:ident info $($property:ident: $property_value:literal),*$(,)?) => {{
        $(let $property = describe_api!(@opg_info_property $property $property_value));*;
        $crate::models::OpgInfo {
            $($property,)*
            ..Default::default()
        }
    }};
    (@opg_info_property title $value:literal) => { $value.to_owned() };
    (@opg_info_property version $value:literal) => { $value.to_owned() };
    (@opg_info_property description $value:literal) => { Some($value.to_owned()) };


    (@opg_property $components:ident tags $($tag:ident$(($description:literal))?),*$(,)?) => {{
        let mut tags = std::collections::BTreeMap::new();
        $(tags.insert(stringify!($tag).to_owned(), $crate::models::OpgTag {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
        }));*;
        tags
    }};

    (@opg_property $components:ident servers $($url:literal$(($description:literal))?),*$(,)?) => {{
        let mut servers = Vec::new();
        $(servers.push($crate::models::OpgServer {
            url: $url.to_owned(),
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
        }));*;
        servers
    }};


    (@opg_property $components:ident paths $(($first_path_segment:tt$( / $path_segment:tt)*): {
        $($properties:tt)*
    }),*$(,)?) => {{
        let mut result = Vec::new();
        $({
            let mut path = Vec::new();
            let mut context = $crate::models::OpgPathValue::default();

            describe_api!(@opg_path_url path context $first_path_segment $($path_segment)*);
            describe_api!(@opg_path_value_properties $components context $($properties)*);

            result.push(($crate::models::OpgPath(path), context));
        };)*
        result
    }};


    (@opg_path_value_properties $components:ident $context:ident $(,)? $field:ident: $value:literal $($other:tt)*) => {
        $context.$field = Some($value.to_owned());
        describe_api!(@opg_path_value_properties $components $context $($other)*)
    };
    (@opg_path_value_properties $components:ident $context:ident $(,)? parameters: { $($parameters:tt)* } $($other:tt)*) => {
        describe_api!(@opg_path_value_parameters $components $context $($parameters)*);
        describe_api!(@opg_path_value_properties $components $context $($other)*)
    };
    (@opg_path_value_properties $components:ident $context:ident $(,)? $method:ident: { $($properties:tt)* } $($other:tt)*) => {
        let mut operation = $crate::models::OpgOperation::default();
        describe_api!(@opg_path_value_operation_properties $components operation $($properties)*);
        $context.operations.insert($crate::models::OpgHttpMethod::$method, operation);

        describe_api!(@opg_path_value_properties $components $context $($other)*)
    };
    (@opg_path_value_properties $components:ident $context:ident $(,)?) => {};


    (@opg_path_value_operation_properties $components:ident $context:ident $(,)? $field:ident: $value:literal $($other:tt)*) => {
        $context.$field = Some($value.to_owned());
        describe_api!(@opg_path_value_operation_properties $components $context $($other)*)
    };
    (@opg_path_value_operation_properties $components:ident $context:ident $(,)? tags: {$($tag:ident),*$(,)?} $($other:tt)*) => {
        $($context.tags.push(stringify!($tag).to_owned()));*;
        describe_api!(@opg_path_value_operation_properties $components $context $($other)*)
    };
    (@opg_path_value_operation_properties $components:ident $context:ident $(,)? parameters: { $($parameters:tt)* } $($other:tt)*) => {
        describe_api!(@opg_path_value_parameters $components $context $($parameters)*);
        describe_api!(@opg_path_value_operation_properties $components $context $($other)*)
    };
    (@opg_path_value_operation_properties $components:ident $context:ident $(,)? body: { $($body:tt)* } $($other:tt)*) => {
        let mut description = None;
        let mut required = true;
        let schema = std::marker::PhantomData::<()>; // just as stub
        describe_api!(@opg_path_value_body_properties $components description required schema $($body)*);
        $context.request_body = Some($crate::models::OpgRequestBody {
           description,
           required,
           schema, // schema must be specified
        });
        describe_api!(@opg_path_value_operation_properties $components $context $($other)*)
    };
    (@opg_path_value_operation_properties $components:ident $context:ident $(,)? $response:literal: $type:tt ($description:literal) $($other:tt)*) => {
        $context.responses.insert($response, $crate::models::OpgResponse {
            description: $description.to_owned(),
            schema: $components.mention::<$type>()
        });
        describe_api!(@opg_path_value_operation_properties $components $context $($other)*)
    };
    (@opg_path_value_operation_properties $components:ident $context:ident $(,)?) => {};


    (@opg_path_value_body_properties $components:ident $description:ident $required:ident $schema:ident $(,)? schema: $type:tt $($other:tt)*) => {
        let $schema = $components.mention::<$type>();
        describe_api!(@opg_path_value_body_properties $components $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $components:ident $description:ident $required:ident $schema:ident $(,)? description: $value:literal $($other:tt)*) => {
        $description = Some($value.to_owned());
        describe_api!(@opg_path_value_body_properties $components $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $components:ident $description:ident $required:ident $schema:ident $(,)? required: $value:literal $($other:tt)*) => {
        $required = $value;
        describe_api!(@opg_path_value_body_properties $components $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $components:ident $description:ident $required:ident $schema:ident $(,)?) => {};


    (@opg_path_value_parameters $components:ident $context:ident (header $name:literal): { $($properties:tt)* } $($other:tt)*) => {{
        let mut parameter = $crate::models::OpgOperationParameter {
            description: None,
            parameter_in: $crate::models::OpgOperationParameterIn::Header,
            required: false,
            schema: Some(String::select_reference(false, &Default::default())),
        };
        describe_api!(@opg_path_value_parameter_properties $components parameter $($properties)*);
        $context.parameters.insert($name.to_owned(), parameter);
    }};
    (@opg_path_value_parameters $components:ident $context:ident (query $name:ident: $type:ty): { $($properties:tt)* } $($other:tt)*) => {{
        let mut parameter = $crate::models::OpgOperationParameter {
            description: None,
            parameter_in: $crate::models::OpgOperationParameterIn::Header,
            required: false,
            schema: Some(<$type as $crate::models::OpgModel>::select_reference(
                false,
                &Default::default(),
            ))
        };
        describe_api!(@opg_path_value_parameter_properties $components parameter $($properties)*);
        $context.parameters.insert(stringify!($name).to_owned(), parameter);
    }};


    (@opg_path_value_parameter_properties $components:ident $context:ident $(,)? description: $value:literal $($other:tt)*) => {
        $context.description = Some($value.to_owned());
        describe_api!(@opg_path_value_parameter_properties $components $context $($other)*)
    };
    (@opg_path_value_parameter_properties $components:ident $context:ident $(,)? required: $value:literal $($other:tt)*) => {
        $context.required = $value;
        describe_api!(@opg_path_value_parameter_properties $components $context $($other)*)
    };
    (@opg_path_value_parameter_properties $components:ident $context:ident $(,)? schema: $type:tt $($other:tt)*) => {
        $context.schema = Some($components.mention::<$type>());
        describe_api!(@opg_path_value_parameter_properties $components $context $($other)*)
    };
    (@opg_path_value_parameter_properties $components:ident $context:ident $(,)?) => {};


    (@opg_path_url $path:ident $context:ident $current:tt $($other:tt)*) => {
        $path.push(describe_api!(@opg_path_url_element $context $current));
        describe_api!(@opg_path_url $path $context $($other)*)
    };
    (@opg_path_url $path:ident $context:ident) => {};

    (@opg_path_url_element $context:ident $segment:literal) => {
        $crate::models::OpgPathElement::Path($segment.to_owned())
    };
    (@opg_path_url_element $context:ident $parameter:ty) => {{
        let name = {
            let name = stringify!($parameter);
            string[..1].to_ascii_lowercase() + &string[1..]
        };
        describe_api!(@opg_path_insert_url_param $context name $parameter)
    }};
    (@opg_path_url_element $context:ident {$name:ident: $parameter:ty}) => {{
        let name = stringify!($name).to_owned();
        describe_api!(@opg_path_insert_url_param $context name $parameter)
    }};
    (@opg_path_insert_url_param $context:ident $name:ident $parameter:ty) => {{
        $context.parameters.insert($name.clone(), $crate::models::OpgOperationParameter {
            description: None,
            parameter_in: $crate::models::OpgOperationParameterIn::Path,
            required: true,
            schema: Some(
                <$parameter as $crate::models::OpgModel>::select_reference(
                    false,
                    &Default::default(),
                )
            )
        });
        $crate::models::OpgPathElement::Parameter($name)
    }}
}
