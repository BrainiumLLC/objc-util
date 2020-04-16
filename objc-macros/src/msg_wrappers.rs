use crate::objc_attr::ObjCAttr;
use quote::ToTokens;
use std::convert::{TryFrom, TryInto};
use syn::{parse, spanned::Spanned};

pub struct MsgWrappers {
    funcs: Vec<MsgWrapper>,
}

impl TryFrom<Vec<syn::ForeignItem>> for MsgWrappers {
    type Error = syn::Error;

    fn try_from(items: Vec<syn::ForeignItem>) -> parse::Result<Self> {
        let funcs = items
            .into_iter()
            .map(|item| match item {
                syn::ForeignItem::Fn(f) => f.try_into(),
                item => Err(syn::Error::new(
                    item.span(),
                    "Expected a function declaration",
                )),
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { funcs })
    }
}

impl ToTokens for MsgWrappers {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for func in &self.funcs {
            func.to_tokens(tokens)
        }
    }
}

struct MsgWrapper {
    attrs: Vec<syn::Attribute>,
    objc_attr: ObjCAttr,
    vis: syn::Visibility,
    fn_token: syn::token::Fn,
    ident: syn::Ident,
    receiver: MsgReceiver,
    args: Vec<(Vec<syn::Attribute>, syn::PatIdent, syn::Type)>,
    output: syn::ReturnType,
    span: proc_macro2::Span,
}

impl TryFrom<syn::ForeignItemFn> for MsgWrapper {
    type Error = syn::Error;

    fn try_from(item: syn::ForeignItemFn) -> parse::Result<Self> {
        let span = item.span();
        let syn::ForeignItemFn {
            mut attrs,
            vis,
            sig,
            semi_token: _,
        } = item;
        let syn::Signature {
            constness,
            asyncness,
            unsafety,
            abi,
            fn_token,
            ident,
            generics,
            paren_token: _,
            inputs,
            variadic,
            output,
        } = sig;

        match constness {
            Some(a) => {
                return Err(syn::Error::new(
                    a.span(),
                    "ObjC binding with `const` is unsupported",
                ))
            }
            None => {}
        }

        match asyncness {
            Some(a) => {
                return Err(syn::Error::new(
                    a.span(),
                    "ObjC binding with `async` is unsupported",
                ))
            }
            None => {}
        }

        match unsafety {
            Some(a) => {
                return Err(syn::Error::new(
                    a.span(),
                    "ObjC bindings are implicitly `unsafe`",
                ))
            }
            None => {}
        }

        match abi {
            Some(a) => {
                return Err(syn::Error::new(
                    a.span(),
                    "ObjC bindings cannot have a custom ABI",
                ))
            }
            None => {}
        }

        match variadic {
            Some(a) => {
                return Err(syn::Error::new(
                    a.span(),
                    "ObjC bindings with variadic arguments are unsupported",
                ))
            }
            None => {}
        }

        match generics.lt_token {
            Some(_) => {
                return Err(syn::Error::new(
                    generics.span(),
                    "ObjC bindings with generics are unsupported",
                ))
            }
            None => {}
        }

        let objc_attr = {
            let objc_attrs: Vec<ObjCAttr> = (0..attrs.len())
                .rev()
                .filter_map(|idx| {
                    if ObjCAttr::is_objc(&attrs[idx]) {
                        Some(attrs.remove(idx).try_into())
                    } else {
                        None
                    }
                })
                .collect::<parse::Result<_>>()?;
            if objc_attrs.len() == 0 {
                return Err(syn::Error::new(
                    ident.span(),
                    "Missing `#[objc(selector = \"xx:xx:xx:\", version = \"#-#-#\")]` attribute",
                ));
            }

            if objc_attrs.len() > 1 {
                return Err(syn::Error::new(
                    objc_attrs[1].span(),
                    "Duplicate `objc` attributes",
                ));
            }
            objc_attrs.into_iter().next().unwrap()
        };

        let sel_arg_count = objc_attr.objc_meth_name.arg_count();
        let rust_arg_count = inputs.len();
        if sel_arg_count != rust_arg_count {
            return Err(syn::Error::new(
                objc_attr.objc_meth_name.span(),
                format!(
                    "ObjC selector has `{}` argument{}, but the rust binding has `{}` argument{}",
                    sel_arg_count,
                    if sel_arg_count == 1 { "" } else { "s" },
                    rust_arg_count,
                    if rust_arg_count == 1 { "" } else { "s" },
                ),
            ));
        }

        let mut args = inputs
            .into_iter()
            .map(|input| match input {
                syn::FnArg::Receiver(r) => Err(syn::Error::new(
                    r.span(),
                    "ObjC bindings with `self` is unsupported",
                )),
                syn::FnArg::Typed(syn::PatType {
                    attrs,
                    pat,
                    colon_token: _,
                    ty,
                }) => {
                    let pat_ident = match *pat {
                        syn::Pat::Ident(pat_ident) => pat_ident,
                        a => {
                            return Err(syn::Error::new(
                                a.span(),
                                "ObjC bindings with destructuring is unsupported",
                            ))
                        }
                    };
                    Ok((attrs, pat_ident, *ty))
                }
            })
            .collect::<parse::Result<Vec<_>>>()?;
        let receiver = args.remove(0).try_into()?;

        Ok(Self {
            attrs,
            objc_attr,
            vis,
            fn_token,
            ident,
            receiver,
            args,
            output,
            span,
        })
    }
}

impl MsgWrapper {
    fn func_impl(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            attrs,
            objc_attr: _,
            vis,
            fn_token,
            ident,
            receiver,
            args,
            output,
            span,
        } = self;
        let void_ptr = syn::Type::Ptr(syn::parse_str("*const std::os::raw::c_void").unwrap());
        let sel_func_ident = self.sel_func_ident();
        let receiver_target_type = receiver.target_type();
        let receiver_name = &receiver.name;
        let message_names = args.iter().map(|(_, ident, _)| ident);
        let message_types = args.iter().map(|(_, _, ty)| match ty {
            // syn::Type::Ptr(p) => &p.elem,
            o => o,
        });
        let middle_message_types = args
            .iter()
            .map(|(_, _, ty)| match ty {
                // syn::Type::Ptr(_) => &void_ptr,
                o => o,
            })
            .collect::<Vec<_>>();
        let all_args_iter = std::iter::once(receiver.as_tuple())
            .chain(args.iter().map(|(attrs, ident, ty)| (attrs, ident, ty)));
        let func_args = all_args_iter
            .clone()
            .map(|(attrs, ident, ty)| {
                quote::quote! {
                    #(#attrs)* #ident: #ty,
                }
            })
            .collect::<proc_macro2::TokenStream>();
        let unit = syn::Type::Tuple(syn::TypeTuple {
            paren_token: Default::default(),
            elems: Default::default(),
        });
        let output_type = match output {
            syn::ReturnType::Default => &unit,
            syn::ReturnType::Type(_, t) => &*t,
        };
        let middle_type = match output_type {
            syn::Type::Ptr(_) => &void_ptr,
            o => o,
        };
        let cfgs = self.objc_attr.versions.os_cfgs();
        let debug_assert_stmt = self.objc_attr.versions.debug_assert_stmt(ident);
        let stream = quote::quote! {
            #(#attrs)*
            #cfgs
            #vis unsafe #fn_token #ident(#func_args) #output
            where
                #receiver_target_type: objc_util::Message,
                #middle_type: objc_util::Encode,
                #(#message_types: objc_util::Encode,)*
            {
                #[deny(improper_ctypes)]
                #[allow(unused)]
                extern "C" { fn #ident(#func_args) #output; }

                #debug_assert_stmt

                let sel = #sel_func_ident();

                if cfg!(debug_assertions) {
                    match objc_util::Message::verify_message::<(#(#middle_message_types,)*), #output_type>(&*#receiver_name, sel) {
                        Ok(()) => {}
                        Err(e) => panic!("Binding error on `{}`: {}", stringify!(#ident), e),
                    }
                }

                let result: #middle_type = match objc_util::Message::send_message(&*#receiver_name, sel, (#(#message_names as #middle_message_types,)*)) {
                    Ok(o) => o,
                    Err(e) => panic!("{}", e),
                };
                result as #output_type
            }
        };
        tokens.extend(crate::assign_group_to_span(stream, *span))
    }

    fn sel_func_ident(&self) -> syn::Ident {
        syn::Ident::new(&format!("_sel_{}", self.ident), self.ident.span())
    }

    fn sel_func_impl(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            attrs: _,
            objc_attr,
            vis,
            fn_token,
            ident,
            receiver: _,
            args: _,
            output: _,
            span,
        } = self;
        let sel_func_ident = self.sel_func_ident();
        let sel_body = objc_attr.objc_meth_name.selector_func_body();
        let cfgs = self.objc_attr.versions.os_cfgs();
        let debug_assert_stmt = self.objc_attr.versions.debug_assert_stmt(ident);
        let stream = quote::quote! {
            #cfgs
            #vis #fn_token #sel_func_ident() -> objc_util::objc::runtime::Sel {
                #debug_assert_stmt
                #sel_body
            }
        };
        tokens.extend(crate::assign_group_to_span(stream, *span))
    }

    fn supported_func_impl(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            attrs,
            objc_attr: _,
            vis,
            fn_token,
            ident,
            receiver: _,
            args: _,
            output: _,
            span,
        } = self;
        let cfgs = self.objc_attr.versions.supported_check();
        let ident = syn::Ident::new(&format!("_supports_{}", ident), ident.span());
        tokens.extend(crate::assign_group_to_span(
            quote::quote! {
                #(#attrs)*
                #vis #fn_token #ident() -> bool {
                    #cfgs
                }
            },
            *span,
        ));
    }

    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.func_impl(tokens);
        self.sel_func_impl(tokens);
        self.supported_func_impl(tokens)
    }
}

struct MsgReceiver {
    attrs: Vec<syn::Attribute>,
    name: syn::PatIdent,
    type_: syn::Type,
}

impl TryFrom<(Vec<syn::Attribute>, syn::PatIdent, syn::Type)> for MsgReceiver {
    type Error = syn::Error;

    fn try_from(
        (attrs, name, type_): (Vec<syn::Attribute>, syn::PatIdent, syn::Type),
    ) -> parse::Result<Self> {
        match type_ {
            syn::Type::Ptr(_) => {}
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "Objc message receivers must be raw pointers",
                ))
            }
        }
        Ok(Self { attrs, name, type_ })
    }
}

impl MsgReceiver {
    fn as_tuple(&self) -> (&Vec<syn::Attribute>, &syn::PatIdent, &syn::Type) {
        (&self.attrs, &self.name, &self.type_)
    }

    fn target_type(&self) -> &syn::Type {
        match &self.type_ {
            syn::Type::Ptr(ptr) => &*ptr.elem,
            _ => unreachable!(),
        }
    }
}
