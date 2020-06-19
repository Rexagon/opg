pub fn wrap_in_const(
    trait_: &str,
    ty: &syn::Ident,
    code: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let dummy_const = if cfg!(underscore_consts) {
        quote::format_ident!("_")
    } else {
        quote::format_ident!("_IMPL_{}_FOR_{}", trait_, unraw(ty))
    };

    let use_opg = quote::quote! {
        #[allow(rust_2018_idioms, clippy::useless_attribute)]
        extern crate opg as _opg;
    };

    quote::quote! {
        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            #use_opg
            #code
        };
    }
}

pub fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}
