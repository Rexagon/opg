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
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::String($crate::ModelString {
                    variants: $crate::macros::FromStrangeTuple::extract(($(vec![$($variants.to_string()),*],)?)),
                    data: ModelSimple {
                        format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                        example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
                    }
                })
            })
        }
    };

    (number => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::Number($crate::ModelSimple {
                    format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                    example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
                })
            })
        }
    };

    (integer => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::Integer($crate::ModelSimple {
                    format: $crate::macros::FromStrangeTuple::extract(($($format.to_string(),)?)),
                    example: $crate::macros::FromStrangeTuple::extract(($($example.to_string(),)?)),
                })
            })
        }
    };

    (boolean => {
        $(description: $description:literal)?
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::Boolean
            })
        }
    };

    (array => {
        $(description: $description:literal)?
        items: ($($property_tail:tt)*)
    }) => {
        $crate::Model {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::Array($crate::ModelArray {
                    items: Box::new(describe_type!(@object_property_value $($property_tail)*))
                })
            })
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
            data: $crate::ModelData::Single($crate::ModelType {
                nullable: false,
                type_description: $crate::ModelTypeDescription::Object($crate::ModelObject {
                    properties,
                    required,
                    ..Default::default()
                })
            })
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
    (generic_simple(nullable$(, ?$sized:ident)?): $($type:tt)+) => {
        impl<T> $crate::OpgModel for $($type)+
        where
            T: $crate::OpgModel$(+ ?$sized)?,
        {
            fn get_schema(cx: &mut $crate::Components) -> $crate::Model {
                <T as $crate::OpgModel>::get_schema_with_params(cx, &$crate::ContextParams {
                    nullable: Some(true),
                    ..Default::default()
                })
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                <T as $crate::OpgModel>::type_name()
            }
        }
    };

    (generic_simple$((?$sized:ident))?: $($type:tt)+) => {
        impl<T> $crate::OpgModel for $($type)+
        where
            T: $crate::OpgModel$(+ ?$sized)?,
        {
            fn get_schema(cx: &mut $crate::Components) -> $crate::Model {
                <T as $crate::OpgModel>::get_schema(cx)
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                <T as $crate::OpgModel>::type_name()
            }
        }
    };

    (generic_tuple: ($($type:ident),+)) => {
        impl<$($type),+> $crate::OpgModel for ($($type),+)
        where
            $($type : $crate::OpgModel),*
        {
            fn get_schema(cx: &mut $crate::Components) -> $crate::Model {
                let item_model = $crate::Model {
                    description: None,
                    data: $crate::ModelData::OneOf($crate::ModelOneOf {
                        one_of: vec![
                            $(cx.mention_schema::<$type>(false, &Default::default())),*
                        ],
                    }),
                };

                describe_type!(array => {
                    items: (raw_model => item_model)
                })
            }

            #[inline]
            fn select_reference(cx: &mut $crate::Components, _: bool, params: &$crate::ContextParams) -> $crate::ModelReference {
                $crate::ModelReference::Inline(Self::get_schema(cx).apply_params(params))
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                None
            }
        }
    };

    (generic_array: $($type:tt)+) => {
        #[allow(clippy::zero_prefixed_literal)]
        impl<T> $crate::OpgModel for $($type)+
        where
            T: $crate::OpgModel,
        {
            fn get_schema(cx: &mut $crate::Components) -> $crate::Model {
                Model {
                    description: None,
                    data: $crate::ModelData::Single($crate::ModelType {
                        nullable: false,
                        type_description: $crate::ModelTypeDescription::Array($crate::ModelArray {
                            items: Box::new(cx.mention_schema::<T>(false, &Default::default())),
                        })
                    }),
                }
            }

            #[inline]
            fn select_reference(cx: &mut $crate::Components, _: bool, params: &$crate::ContextParams) -> $crate::ModelReference {
                $crate::ModelReference::Inline(Self::get_schema(cx).apply_params(params))
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                None
            }
        }
    };

    (generic_dictionary: $($type:tt)+) => {
        impl<K, T> $crate::OpgModel for $($type)+
        where
            T: $crate::OpgModel,
            K: serde::ser::Serialize,
        {
            fn get_schema(cx: &mut $crate::Components) -> $crate::Model {
                Model {
                    description: None,
                    data: $crate::ModelData::Single($crate::ModelType {
                        nullable: false,
                        type_description: $crate::ModelTypeDescription::Object($crate::ModelObject {
                            additional_properties: Some(Box::new(cx.mention_schema::<T>(false, &Default::default()))),
                            ..Default::default()
                        })
                    }),
                }
            }

            #[inline]
            fn select_reference(cx: &mut $crate::Components, _: bool, params: &$crate::ContextParams) -> $crate::ModelReference {
                $crate::ModelReference::Inline(Self::get_schema(cx).apply_params(params))
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                None
            }
        }
    };

    ($serialized_type:ident$(($format:literal))?: $($type:tt)+ ) => {
        impl $crate::OpgModel for $($type)+ {
            fn get_schema(cx: &mut $crate::Components) -> Model {
                describe_type!($serialized_type => {
                    $(format: $format)?
                })
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                <T as $crate::OpgModel>::type_name()
            }
        }
    };

    ($serialized_type:ident(always_inline$(, $format:literal)?): $($type:tt)+) => {
        impl $crate::OpgModel for $($type)+ {
            fn get_schema(_: &mut $crate::Components) -> Model {
                describe_type!($serialized_type => {
                    $(format: $format)?
                })
            }

            #[inline]
            fn select_reference(cx: &mut $crate::Components, _: bool, params: &$crate::ContextParams) -> $crate::ModelReference {
                $crate::ModelReference::Inline(Self::get_schema(cx).apply_params(params))
            }

            #[inline]
            fn type_name() -> Option<&'static str> {
                None
            }
        }
    };
);

#[macro_export]
macro_rules! describe_api {
    ($($property:ident: {$($property_value:tt)*}),*$(,)?) => {{
        let mut result = $crate::models::Opg::default();

        $(describe_api!(@opg_property result $property $($property_value)*));+;

        result
    }};


    (@opg_property $result:ident info $($property:ident: $property_value:literal),*$(,)?) => {{
        $(let $property = describe_api!(@opg_info_property $property $property_value));*;
        $result.info = $crate::models::Info {
            $($property,)*
            ..Default::default()
        };
    }};
    (@opg_info_property title $value:literal) => { $value.to_owned() };
    (@opg_info_property version $value:literal) => { $value.to_owned() };
    (@opg_info_property description $value:literal) => { Some($value.to_owned()) };


    (@opg_property $result:ident tags $($tag:ident$(($description:literal))?),*$(,)?) => {{
        $($result.tags.insert(stringify!($tag).to_owned(), $crate::models::Tag {
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
        }));*;
    }};


    (@opg_property $result:ident servers $($url:literal$(($description:literal))?),*$(,)?) => {{
        $($result.servers.push($crate::models::Server {
            url: $url.to_owned(),
            description: $crate::macros::FromStrangeTuple::extract(($($description.to_string(),)?)),
        }));*;
    }};


    (@opg_property $result:ident security_schemes $($schemes:tt)*) => {
        describe_api!(@opg_security_scheme $result $($schemes)*,)
    };
    (@opg_security_scheme $result:ident $scheme:ident, $($other:tt)*) => {
        $result.components.security_schemes.insert(stringity!($scheme).to_owned(), $scheme);
        describe_api!(@opg_security_scheme $result $($other)*)
    };
    (@opg_security_scheme $result:ident (http $name:literal): {$($properties:tt)+}, $($other:tt)*) => {
        {
            let scheme = $crate::models::ParameterNotSpecified;
            let mut bearer_format: Option<String> = None;

            describe_api!(@opg_security_scheme_http scheme bearer_format $($properties)*,);

            let http_security_scheme = match scheme {
                $crate::HttpSecuritySchemeKind::Basic => $crate::models::HttpSecurityScheme::Basic,
                $crate::HttpSecuritySchemeKind::Bearer => $crate::models::HttpSecurityScheme::Bearer {
                    format: bearer_format,
                },
            };

            $result.components.security_schemes.insert($name.to_owned(), $crate::models::SecurityScheme::Http(http_security_scheme));
        };
        describe_api!(@opg_security_scheme $result $($other)*)
    };
    (@opg_security_scheme $result:ident (apiKey $name:literal): {$($properties:tt)+}, $($other:tt)*) => {
        {
            let mut parameter_in = $crate::models::ParameterIn::Header;
            let name = $crate::models::ParameterNotSpecified;

            describe_api!(@opg_security_scheme_api_key parameter_in name $($properties)*,);

            let scheme = $crate::models::ApiKeySecurityScheme {
                parameter_in,
                name,
            };

            $result.components.security_schemes.insert($name.to_owned(), $crate::models::SecurityScheme::ApiKey(scheme));
        };
        describe_api!(@opg_security_scheme $result $($other)*)
    };
    (@opg_security_scheme $result:ident $(,)?) => {};


    (@opg_security_scheme_http $scheme:ident $bearer_format:ident scheme: $scheme_kind:ident, $($other:tt)*) => {
        let $scheme = $crate::models::HttpSecuritySchemeKind::$scheme_kind;
        describe_api!(@opg_security_scheme_http $scheme $bearer_format $($other)*)
    };
    (@opg_security_scheme_http $scheme:ident $bearer_format:ident bearer_format: $format:literal, $($other:tt)*) => {
        let $bearer_format = Some($format.to_owned());
        describe_api!(@opg_security_scheme_http $scheme $bearer_format $($other)*)
    };
    (@opg_security_scheme_http $scheme:ident $bearer_format:ident $(,)?) => {};


    (@opg_security_scheme_api_key $parameter_in:ident $name:ident parameter_in: $value:ident, $($other:tt)*) => {
        $parameter_in = $crate::models::ParameterIn::$value;
        describe_api!(@opg_security_scheme_api_key $parameter_in $name $($other)*)
    };
    (@opg_security_scheme_api_key $parameter_in:ident $name:ident name: $value:literal, $($other:tt)*) => {
        let $name = $value.to_owned();
        describe_api!(@opg_security_scheme_api_key $parameter_in $name $($other)*)
    };
    (@opg_security_scheme_api_key $parameter_in:ident $name:ident $(,)?) => {};


    (@opg_property $result:ident paths $(($first_path_segment:tt$( / $path_segment:tt)*): {
        $($properties:tt)*
    }),*$(,)?) => {{
        $({
            let mut path = Vec::new();
            let mut context = $crate::models::PathValue::default();

            describe_api!(@opg_path_url path $result context $first_path_segment $($path_segment)*);
            describe_api!(@opg_path_value_properties $result context $($properties)*,);

            $result.paths.push(($crate::models::Path(path), context));
        };)*
    }};


    (@opg_path_value_properties $result:ident $context:ident $field:ident: $value:literal, $($other:tt)*) => {
        $context.$field = Some($value.to_owned());
        describe_api!(@opg_path_value_properties $result $context $($other)*)
    };
    (@opg_path_value_properties $result:ident $context:ident parameters: { $($parameters:tt)* }, $($other:tt)*) => {
        describe_api!(@opg_path_value_parameters $result $context $($parameters)*,);
        describe_api!(@opg_path_value_properties $result $context $($other)*)
    };
    (@opg_path_value_properties $result:ident $context:ident $method:ident: { $($properties:tt)* }, $($other:tt)*) => {
        let mut operation = $crate::models::Operation::default();
        describe_api!(@opg_path_value_operation_properties $result operation $($properties)*,);
        $context.operations.insert($crate::models::HttpMethod::$method, operation);

        describe_api!(@opg_path_value_properties $result $context $($other)*)
    };
    (@opg_path_value_properties $result:ident $context:ident $(,)?) => {};


    (@opg_path_value_operation_properties $result:ident $context:ident $field:ident: $value:literal, $($other:tt)*) => {
        $context.$field = Some($value.to_owned());
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident tags: {$($tag:ident),*$(,)?}, $($other:tt)*) => {
        $($context.tags.push(stringify!($tag).to_owned()));*;
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident parameters: { $($parameters:tt)* }, $($other:tt)*) => {
        describe_api!(@opg_path_value_parameters $result $context $($parameters)*,);
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident security: { $($security:tt)* }, $($other:tt)*) => {
        describe_api!(@opg_path_value_security $result $context $($security)*,);
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident body: { $($body:tt)* }, $($other:tt)*) => {
        let mut description = None;
        let mut required = true;
        let schema = $crate::models::ParameterNotSpecified;
        describe_api!(@opg_path_value_body_properties $result description required schema $($body)*,);
        $context.request_body = Some($crate::models::RequestBody {
           description: description.or(Some(String::new())),
           required,
           schema, // schema must be specified
        });
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident body: $type:path, $($other:tt)*) => {
        $context.request_body = Some($crate::models::RequestBody {
           description: Some(String::new()),
           required: true,
           schema: $result.components.mention_schema::<$type>(false, &Default::default()),
        });
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident $response:literal($description:literal): $type:path, $($other:tt)*) => {
        $context.responses.insert($response, $crate::models::Response {
            description: $description.to_owned(),
            schema: $result.components.mention_schema::<$type>(false, &Default::default())
        });
        describe_api!(@opg_path_value_operation_properties $result $context $($other)*)
    };
    (@opg_path_value_operation_properties $result:ident $context:ident $(,)?) => {};


    (@opg_path_value_security $result:ident $context:ident $($security:tt$([$($role:literal),*])?)&&+, $($other:tt)*) => {
        {
            let mut security = std::collections::BTreeMap::new();
            $(describe_api!(@opg_path_value_security_item $result security $security$([$($role),*])?));*;
            $context.security.push(security);
        }
        describe_api!(@opg_path_value_security $result $context $($other)*)
    };
    (@opg_path_value_security $result:ident $context:ident $(,)*) => {};


    (@opg_path_value_security_item $result:ident $context:ident $security:literal$([$($role:literal),*])?) => {
        $context.insert($security.to_owned(), vec![$($($role),*)?])
    };
    (@opg_path_value_security_item $result:ident $context:ident $security:ident$([$($role:literal),*])?) => {
        $context.insert($result.components.mention_security_scheme(stringify!($security).to_owned(), &$security), vec![$($($role),*)?])
    };


    (@opg_path_value_body_properties $result:ident $description:ident $required:ident $schema:ident schema: $type:path, $($other:tt)*) => {
        let $schema = $result.components.mention_schema::<$type>(false, &Default::default());
        describe_api!(@opg_path_value_body_properties $result $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $result:ident $description:ident $required:ident $schema:ident description: $value:literal, $($other:tt)*) => {
        $description = Some($value.to_owned());
        describe_api!(@opg_path_value_body_properties $result $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $result:ident $description:ident $required:ident $schema:ident required: $value:literal, $($other:tt)*) => {
        $required = $value;
        describe_api!(@opg_path_value_body_properties $result $description $required $schema $($other)*)
    };
    (@opg_path_value_body_properties $result:ident $description:ident $required:ident $schema:ident $(,)?) => {};


    (@opg_path_value_parameters $result:ident $context:ident (header $name:literal): { $($properties:tt)* }, $($other:tt)*) => {
        {
            let mut parameter = $crate::models::OperationParameter {
                description: None,
                parameter_in: $crate::models::ParameterIn::Header,
                required: true,
                schema: Some($result.components.mention_schema::<String>(false, &Default::default())),
            };
            describe_api!(@opg_path_value_parameter_properties $result parameter $($properties)*,);
            $context.parameters.insert($name.to_owned(), parameter);
        }
        describe_api!(@opg_path_value_parameters $result $context $($other)*)
    };
    (@opg_path_value_parameters $result:ident $context:ident (header $name:literal), $($other:tt)*) => {
        {
            let mut parameter = $crate::models::OperationParameter {
                description: None,
                parameter_in: $crate::models::ParameterIn::Header,
                required: true,
                schema: Some($result.components.mention_schema::<String>(false, &Default::default())),
            };
            $context.parameters.insert($name.to_owned(), parameter);
        }
        describe_api!(@opg_path_value_parameters $result $context $($other)*)
    };
    (@opg_path_value_parameters $result:ident $context:ident (query $name:ident: $type:path): { $($properties:tt)* }, $($other:tt)*) => {
        {
            let mut parameter = $crate::models::OperationParameter {
                description: None,
                parameter_in: $crate::models::ParameterIn::Query,
                required: false,
                schema: Some($result.components.mention_schema::<$type>(false, &Default::default())),
            };
            describe_api!(@opg_path_value_parameter_properties $result parameter $($properties)*,);
            $context.parameters.insert(stringify!($name).to_owned(), parameter);
        }
        describe_api!(@opg_path_value_parameters $result $context $($other)*)
    };
    (@opg_path_value_parameters $result:ident $context:ident (query $name:ident: $type:path), $($other:tt)*) => {
        {
            let mut parameter = $crate::models::OperationParameter {
                description: None,
                parameter_in: $crate::models::ParameterIn::Query,
                required: false,
                schema: Some($result.components.mention_schema::<$type>(false, &Default::default())),
            };
            $context.parameters.insert(stringify!($name).to_owned(), parameter);
        }
        describe_api!(@opg_path_value_parameters $result $context $($other)*)
    };
    (@opg_path_value_parameters $result:ident $context:ident $(,)?) => {};


    (@opg_path_value_parameter_properties $result:ident $context:ident description: $value:literal, $($other:tt)*) => {
        $context.description = Some($value.to_owned());
        describe_api!(@opg_path_value_parameter_properties $result $context $($other)*)
    };
    (@opg_path_value_parameter_properties $result:ident $context:ident required: $value:literal, $($other:tt)*) => {
        $context.required = $value;
        describe_api!(@opg_path_value_parameter_properties $result $context $($other)*)
    };
    (@opg_path_value_parameter_properties $result:ident $context:ident schema: $type:path, $($other:tt)*) => {
        $context.schema = Some($result.components.mention_schema::<$type>(false, &Default::default()));
        describe_api!(@opg_path_value_parameter_properties $result $context $($other)*)
    };
    (@opg_path_value_parameter_properties $result:ident $context:ident $(,)?) => {};


    (@opg_path_url $path:ident $result:ident $context:ident $current:tt $($other:tt)*) => {
        $path.push(describe_api!(@opg_path_url_element $result $context $current));
        describe_api!(@opg_path_url $path $result $context $($other)*)
    };
    (@opg_path_url $path:ident $result:ident $context:ident) => {};

    (@opg_path_url_element $result:ident $context:ident $segment:literal) => {
        $crate::models::PathElement::Path($segment.to_owned())
    };
    (@opg_path_url_element $result:ident $context:ident $parameter:path) => {{
        let name = {
            let name = stringify!($parameter);
            name[..1].to_ascii_lowercase() + &name[1..]
        };
        describe_api!(@opg_path_insert_url_param $result $context name $parameter)
    }};
    (@opg_path_url_element $result:ident $context:ident {$name:ident: $parameter:path}) => {{
        let name = stringify!($name).to_owned();
        describe_api!(@opg_path_insert_url_param $result $context name $parameter)
    }};
    (@opg_path_insert_url_param $result:ident $context:ident $name:ident $parameter:path) => {{
        $context.parameters.insert($name.clone(), $crate::models::OperationParameter {
            description: None,
            parameter_in: $crate::models::ParameterIn::Path,
            required: true,
            schema: Some($result.components.mention_schema::<$parameter>(false, &Default::default()))
        });
        $crate::models::PathElement::Parameter($name)
    }}
}
