use crate::calculate_hash;
use std::convert::{TryFrom, TryInto};
use syn::{
    parse::{self, Parse, ParseStream},
    spanned::Spanned,
};

pub struct ObjCSelector {
    pub objc_meth_name: ObjCMethName,
}

impl TryFrom<syn::NestedMeta> for ObjCSelector {
    type Error = syn::Error;

    fn try_from(nested_meta: syn::NestedMeta) -> parse::Result<Self> {
        let span = nested_meta.span();
        match nested_meta {
            syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => match nv.path.get_ident() {
                Some(ident) if ident == "selector" => {
                    return Ok(Self {
                        objc_meth_name: nv.lit.try_into()?,
                    })
                }
                _ => {}
            },
            _ => {}
        };
        Err(syn::Error::new(span, "Expected `selector = \"xx:xx:xx:\"`"))
    }
}

pub struct ObjCMethName {
    name: syn::punctuated::Punctuated<syn::Ident, syn::token::Colon>,
}

impl TryFrom<syn::Lit> for ObjCMethName {
    type Error = syn::Error;

    fn try_from(lit: syn::Lit) -> parse::Result<Self> {
        match lit {
            syn::Lit::Str(s) => s.parse(),
            o => Err(syn::Error::new(
                o.span(),
                "Expected an ObjC method name (e.g. `fooBar:with:`)",
            )),
        }
    }
}

impl Spanned for ObjCMethName {
    fn span(&self) -> proc_macro2::Span {
        self.name.span()
    }
}

impl Parse for ObjCMethName {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let name = syn::punctuated::Punctuated::parse_terminated(input)?;
        if name.len() == 0 || name.len() > 1 && !name.trailing_punct() {
            Err(syn::Error::new(
                name.span(),
                "Invalid ObjC method name. Are you missing a trailing `:`?",
            ))
        } else {
            Ok(Self { name })
        }
    }
}

impl ObjCMethName {
    pub fn arg_count(&self) -> usize {
        self.name.len() - (!self.name.trailing_punct()) as usize + 1
    }

    pub fn as_string(&self) -> String {
        self.name
            .pairs()
            .map(|pair| {
                if pair.punct().is_some() {
                    format!("{}:", pair.value())
                } else {
                    format!("{}", pair.value())
                }
            })
            .collect()
    }

    pub fn selector_func_body(&self) -> proc_macro2::TokenStream {
        if cfg!(feature = "compile-time") {
            let mut selector_string = self.as_string();
            let random_id = &format!(
                "{}",
                calculate_hash(&format!("{}{:?}", selector_string, self.span()))
            );
            let meth_name_export_name = [
                "\x01L_OBJC_METH_VAR_NAME_.__objc_util_meth.",
                random_id,
                ".",
                &selector_string,
            ]
            .concat();

            let sel_ref_export_name = [
                "\x01L_OBJC_SELECTOR_REFERENCES_.__objc_util_sel.",
                random_id,
                ".",
                &selector_string,
            ]
            .concat();
            selector_string.push('\x00');
            let selector = syn::LitByteStr::new(selector_string.as_bytes(), self.span());
            let selector_len = selector_string.len();

            quote::quote! {
                #[link_section = "__TEXT,__objc_methname,cstring_literals"]
                #[export_name = #meth_name_export_name]
                static METH_NAME: [u8; #selector_len] = * #selector;

                #[link_section = "__DATA,__objc_selrefs,literal_pointers,no_dead_strip"]
                #[export_name = #sel_ref_export_name]
                static SEL_REF: &'static [u8; #selector_len] = &METH_NAME;

                let sel: objc_util::runtime::Sel = unsafe {
                    core::mem::transmute::<&'static [u8; #selector_len], _>(core::ptr::read_volatile(
                        &SEL_REF as *const &'static [u8; #selector_len]
                    ))
                };
                sel
            }
        } else {
            let selector = self.as_string();
            let lit = syn::LitStr::new(&selector, self.span());
            let tokens = lit.parse::<proc_macro2::TokenStream>().unwrap();
            quote::quote! {
                {
                    use objc_util::objc::sel_impl;
                    objc_util::objc::sel!(#tokens)
                }
            }
        }
    }
}
