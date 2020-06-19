pub mod macros;
pub mod models;

pub use macros::*;
pub use models::*;
pub use opg_derive::*;

pub const OPENAPI_VERSION: &'static str = "3.0.1";
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
}

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
}
