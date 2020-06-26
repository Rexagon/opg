pub mod macros;
pub mod models;

pub use macros::*;
pub use models::*;
pub use opg_derive::OpgModel;

pub const OPENAPI_VERSION: &str = "3.0.3";
pub const SCHEMA_REFERENCE_PREFIX: &str = "#/components/schemas/";

impl_opg_model!(string(always_inline): char);
impl_opg_model!(string(always_inline): str);
impl_opg_model!(string(always_inline): String);

impl_opg_model!(integer(always_inline): i8);
impl_opg_model!(integer(always_inline): u8);
impl_opg_model!(integer(always_inline): i16);
impl_opg_model!(integer(always_inline): u16);
impl_opg_model!(integer(always_inline): i32);
impl_opg_model!(integer(always_inline): u32);
impl_opg_model!(integer(always_inline): i64);
impl_opg_model!(integer(always_inline): u64);
impl_opg_model!(integer(always_inline): isize);
impl_opg_model!(integer(always_inline): usize);

impl_opg_model!(number(always_inline): f32);
impl_opg_model!(number(always_inline): f64);

impl_opg_model!(boolean(always_inline): bool);

impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicI8);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicU8);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicI16);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicU16);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicI32);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicU32);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicI64);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicU64);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicIsize);
impl_opg_model!(integer(always_inline): std::sync::atomic::AtomicUsize);

impl_opg_model!(boolean(always_inline): std::sync::atomic::AtomicBool);

impl_opg_model!(generic_simple: &T);
impl_opg_model!(generic_simple: &mut T);
impl_opg_model!(generic_simple: (T,));
impl_opg_model!(generic_simple: Box<T>);
impl_opg_model!(generic_simple: std::rc::Rc<T>);
impl_opg_model!(generic_simple: std::sync::Arc<T>);
impl_opg_model!(generic_simple: std::cell::Cell<T>);
impl_opg_model!(generic_simple: std::cell::RefCell<T>);

impl_opg_model!(generic_simple(nullable): Option<T>);

impl_opg_model!(generic_tuple: (T1, T2));
impl_opg_model!(generic_tuple: (T1, T2, T3));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5, T6));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5, T6, T7));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5, T6, T7, T8));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9));
impl_opg_model!(generic_tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10));

impl_opg_model!(generic_array: [T]);
impl_opg_model!(generic_array: Vec<T>);
impl_opg_model!(generic_array: std::collections::HashSet<T>);
impl_opg_model!(generic_array: std::collections::LinkedList<T>);
impl_opg_model!(generic_array: std::collections::VecDeque<T>);
impl_opg_model!(generic_array: std::collections::BinaryHeap<T>);

macro_rules! array_impls {
    ($($len:tt)+) => {
        $(impl_opg_model!(generic_array: [T; $len]);)*
    };
}
array_impls! {
    01 02 03 04 05 06 07 08 09 10
    11 12 13 14 15 16 17 18 19 20
    21 22 23 24 25 26 27 28 29 30
    31 32
}

impl_opg_model!(generic_dictionary: std::collections::HashMap<K, T>);
impl_opg_model!(generic_dictionary: std::collections::BTreeMap<K, T>);

#[cfg(feature = "uuid")]
impl OpgModel for uuid::Uuid {
    fn get_structure(_: &mut OpgComponents) -> Model {
        Model {
            description: Some("UUID ver. 4 [rfc](https://tools.ietf.org/html/rfc4122)".to_owned()),
            data: ModelData::Single(ModelType {
                nullable: false,
                type_description: ModelTypeDescription::String(ModelString {
                    variants: None,
                    data: ModelSimple {
                        format: Some("uuid".to_owned()),
                        example: Some("00000000-0000-0000-0000-000000000000".to_owned()),
                    },
                }),
            }),
        }
    }

    #[inline(always)]
    fn select_reference(cx: &mut OpgComponents, _: bool, params: &ContextParams) -> ModelReference {
        ModelReference::Inline(Self::get_structure(cx).apply_params(params))
    }

    #[inline(always)]
    fn get_type_name() -> Option<&'static str> {
        None
    }
}

#[cfg(feature = "chrono")]
impl OpgModel for chrono::NaiveDateTime {
    fn get_structure(_: &mut OpgComponents) -> Model {
        Model {
            description: Some("Datetime".to_owned()),
            data: ModelData::Single(ModelType {
                nullable: false,
                type_description: ModelTypeDescription::String(ModelString {
                    variants: None,
                    data: ModelSimple {
                        format: Some("YYYY-MM-DDThh:mm:ss.sTZD".to_owned()),
                        example: Some("2020-06-26T14:04:20.730045106Z".to_owned()),
                    },
                }),
            }),
        }
    }

    #[inline(always)]
    fn select_reference(cx: &mut OpgComponents, _: bool, params: &ContextParams) -> ModelReference {
        ModelReference::Inline(Self::get_structure(cx).apply_params(params))
    }

    #[inline(always)]
    fn get_type_name() -> Option<&'static str> {
        None
    }
}
