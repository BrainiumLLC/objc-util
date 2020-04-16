use crate::msg_wrappers::MsgWrappers;
use quote::ToTokens;
use std::convert::TryInto as _;
use syn::parse::{self, Parse, ParseStream};

pub struct ExternObjc {
    msg_wrappers: MsgWrappers,
}

impl Parse for ExternObjc {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let block = input.parse::<syn::ItemForeignMod>()?;
        if let Some(name) = block.abi.name {
            if name.value() != "ObjC" {
                return Err(syn::Error::new(name.span(), "Expected `ObjC`"));
            }
        }
        let msg_wrappers = block.items.try_into()?;
        Ok(ExternObjc { msg_wrappers })
    }
}

impl ToTokens for ExternObjc {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { msg_wrappers } = self;
        tokens.extend(quote::quote! {
            #msg_wrappers
        })
    }
}
