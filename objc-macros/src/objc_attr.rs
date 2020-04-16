use crate::{
    objc_selector::{ObjCMethName, ObjCSelector},
    os_versions::OSVersions,
};
use std::convert::{TryFrom, TryInto};
use syn::{parse, spanned::Spanned};

pub struct ObjCAttr {
    pub objc_meth_name: ObjCMethName,
    pub versions:       OSVersions,
    span:               proc_macro2::Span,
}

impl ObjCAttr {
    pub fn is_objc(attr: &syn::Attribute) -> bool {
        match attr.path.get_ident() {
            Some(ident) => {
                if ident == "objc" {
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

impl Spanned for ObjCAttr {
    fn span(&self) -> proc_macro2::Span {
        self.span
    }
}

impl TryFrom<syn::Attribute> for ObjCAttr {
    type Error = syn::Error;

    fn try_from(attr: syn::Attribute) -> parse::Result<Self> {
        let span = attr.span();
        match attr.style {
            syn::AttrStyle::Inner(_) => {
                return Err(syn::Error::new(
                    attr.span(),
                    "Expected an Outer attribute. Remove the `!`.",
                ))
            }
            _ => {}
        }
        let list = match attr.parse_meta()? {
            syn::Meta::List(list) => list,
            o => {
                return Err(syn::Error::new(
                    o.span(),
                    "Expected an Outer attribute. Remove the `!`.",
                ))
            }
        };
        if list.nested.len() < 2 {
            return Err(syn::Error::new(
                list.nested.span(),
                "Expected `selector = \"xx:xx:xx:\", OS_NAME = \"#-#-#\"`.",
            ));
        }
        let mut iter = list.nested.into_iter();
        let sig: ObjCSelector = iter.next().unwrap().try_into()?;
        let versions = iter.try_into()?;

        Ok(Self {
            objc_meth_name: sig.objc_meth_name,
            versions,
            span,
        })
    }
}
