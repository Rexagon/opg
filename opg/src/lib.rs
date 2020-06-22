pub mod macros;
pub mod models;

pub use macros::*;
pub use models::*;
pub use opg_derive::OpgModel;

pub const OPENAPI_VERSION: &'static str = "3.0.3";
pub const SCHEMA_REFERENCE_PREFIX: &'static str = "#/components/schemas/";

impl_opg_model!(String => string always_inline);

impl_opg_model!(i8 => integer always_inline);
impl_opg_model!(u8 => integer always_inline);
impl_opg_model!(i16 => integer always_inline);
impl_opg_model!(u16 => integer always_inline);
impl_opg_model!(i32 => integer always_inline);
impl_opg_model!(u32 => integer always_inline);
impl_opg_model!(i64 => integer always_inline);
impl_opg_model!(u64 => integer always_inline);

impl_opg_model!(f32 => number always_inline);
impl_opg_model!(f64 => number always_inline);

impl_opg_model!(bool => boolean always_inline);

impl_opg_model!(generic_tuple(T1, T2));
impl_opg_model!(generic_tuple(T1, T2, T3));
impl_opg_model!(generic_tuple(T1, T2, T3, T4));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5, T6));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5, T6, T7));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5, T6, T7, T8));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5, T6, T7, T8, T9));
impl_opg_model!(generic_tuple(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10));

impl<T> OpgModel for Vec<T>
where
    T: OpgModel,
{
    fn get_structure() -> Model {
        Model {
            description: None,
            data: ModelData::Single(ModelTypeDescription::Array(ModelArray {
                items: Box::new(ModelReference::Inline(T::get_structure())),
            })),
        }
    }

    #[inline(always)]
    fn select_reference(_: bool, inline_params: &ContextParams) -> ModelReference {
        Self::inject(InjectReference::Inline(inline_params))
    }
}

#[cfg(feature = "uuid")]
impl OpgModel for uuid::Uuid {
    fn get_structure() -> Model {
        Model {
            description: Some(format!(
                "UUID ver. 4 [rfc](https://tools.ietf.org/html/rfc4122)"
            )),
            data: ModelData::Single(ModelTypeDescription::String(ModelString {
                variants: None,
                data: ModelSimple {
                    format: Some(format!("uuid")),
                    example: Some(format!("00000000-0000-0000-0000-000000000000")),
                },
            })),
        }
    }

    #[inline(always)]
    fn select_reference(_: bool, inline_params: &ContextParams) -> ModelReference {
        Self::inject(InjectReference::Inline(inline_params))
    }
}
