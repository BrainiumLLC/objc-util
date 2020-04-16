extern crate proc_macro;

mod extern_objc;
mod framework;
mod msg_wrappers;
mod objc_attr;
mod objc_selector;
mod os_versions;

use crate::{extern_objc::ExternObjc, framework::Framework};
use proc_macro::TokenStream;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher as _},
};
use syn::{
    parse::{self, Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

pub(crate) fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub(crate) fn assign_group_to_span(
    stream: proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    stream
        .into_iter()
        .map(|mut tree| {
            tree.set_span(span);
            tree
        })
        .collect()
}

struct OSSupports {
    supports_func: syn::Path,
}

impl Parse for OSSupports {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let mut supports_func: syn::Path = input.parse()?;
        let mut last = supports_func.segments.last_mut().unwrap();
        last.ident = syn::Ident::new(&format!("_supports_{}", last.ident), last.ident.span());
        Ok(Self { supports_func })
    }
}

#[proc_macro]
pub fn os_supports_impl(input: TokenStream) -> TokenStream {
    let OSSupports { supports_func } = parse_macro_input!(input);
    let parsed = assign_group_to_span(
        quote::quote! {
            fn proc_macro_support_wrapper() -> bool {
                #supports_func()
            }
        },
        supports_func.span(),
    );
    parsed.into()
}

struct SelectorFunc {
    sel_func: syn::Path,
}

impl Parse for SelectorFunc {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let mut sel_func: syn::Path = input.parse()?;
        let mut last = sel_func.segments.last_mut().unwrap();
        last.ident = syn::Ident::new(&format!("_sel_{}", last.ident), last.ident.span());
        Ok(Self { sel_func })
    }
}

#[proc_macro]
pub fn sel_impl(input: TokenStream) -> TokenStream {
    let SelectorFunc { sel_func } = parse_macro_input!(input);
    let parsed = assign_group_to_span(
        quote::quote! {
            fn proc_macro_support_wrapper() -> objc_util::runtime::Sel {
                #sel_func()
            }
        },
        sel_func.span(),
    );
    parsed.into()
}

#[cfg(feature = "compile-time")]
#[proc_macro]
pub fn class_impl(input: TokenStream) -> TokenStream {
    let class: syn::Ident = parse_macro_input!(input);
    let random_id = &format!(
        "{}",
        calculate_hash(&format!("{}{:?}", class, class.span()))
    );
    let class_string = format!("{}", class);

    let class_name = ["\x01_OBJC_CLASS_$_", &class_string].concat();

    let class_ref_export_name = [
        "\x01L_OBJC_CLASSLIST_REFERENCES_$_.",
        random_id,
        ".",
        &class_string,
    ]
    .concat();

    let parsed = assign_group_to_span(
        quote::quote! {
            fn proc_macro_support_wrapper() -> &'static objc_util::runtime::Class {
                #[link(name = "objc", kind = "dylib")]
                extern {
                    fn objc_opt_class(name: *const std::os::raw::c_void) -> *mut std::os::raw::c_void;
                }
                extern {
                    #[link_name = #class_name]
                    static CLASS_NAME: u8;
                }

                #[link_section = "__DATA,__objc_classrefs,regular,no_dead_strip"]
                #[export_name = #class_ref_export_name]
                static CLASS_REF: &'static u8 = unsafe { &CLASS_NAME };

                let class: &'static objc_util::runtime::Class = unsafe {
                    core::mem::transmute(
                        objc_opt_class(core::ptr::read_volatile(&CLASS_REF) as *const _ as *const _)
                    )
                };
                class
            }
        },
        class.span(),
    );
    parsed.into()
}

#[cfg(not(feature = "compile-time"))]
#[proc_macro]
pub fn class_impl(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = parse_macro_input!(input);
    let tokens = quote::quote! {
        fn proc_macro_support_wrapper() -> &'static objc_util::runtime::Class {
            objc_util::objc::class!(#input)
        }
    };
    tokens.into()
}

#[proc_macro_attribute]
pub fn extern_objc(args: TokenStream, input: TokenStream) -> TokenStream {
    let framework: Framework = parse_macro_input!(args);
    let data: ExternObjc = parse_macro_input!(input);
    let parsed = quote::quote! {
        #framework
        #data
    };
    parsed.into()
}
