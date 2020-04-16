use quote::ToTokens;
use syn::parse::{self, Parse, ParseStream};

pub struct Framework {
    name: syn::LitStr,
}

impl Parse for Framework {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let framework = input.parse::<syn::Ident>()?;
        if framework != "framework" {
            return Err(syn::Error::new(framework.span(), "Expected `framework`"));
        }
        let _ = input.parse::<syn::Token![=]>()?;
        let name = input.parse()?;
        Ok(Framework { name })
    }
}

impl ToTokens for Framework {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        tokens.extend(quote::quote! {
            #[link(name = #name, kind = "framework")]
            extern "C" {}
        })
    }
}
