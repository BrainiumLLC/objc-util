use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};
use syn::{
    parse,
    punctuated::Punctuated,
    spanned::Spanned,
};

#[derive(Default)]
pub struct OSVersions {
    versions: HashMap<OS, (Version, proc_macro2::Span)>,
}

impl TryFrom<syn::punctuated::IntoIter<syn::NestedMeta>> for OSVersions {
    type Error = syn::Error;

    fn try_from(iter: syn::punctuated::IntoIter<syn::NestedMeta>) -> parse::Result<Self> {
        let os_version = iter
            .map(|os_version| os_version.try_into())
            .collect::<parse::Result<Vec<OSVersion>>>()?;
        let mut versions = OSVersions::new();
        for os_version in os_version {
            let span = os_version.span();
            let old = versions.insert(os_version);
            if let Some(old) = old {
                return Err(syn::Error::new(
                    span,
                    &format!("Duplicate `{}` keys", old.os.as_str()),
                ));
            }
        }
        Ok(versions)
    }
}

impl OSVersions {
    fn new() -> Self {
        Default::default()
    }

    fn insert(&mut self, OSVersion { os, version, span }: OSVersion) -> Option<OSVersion> {
        self.versions
            .insert(os, (version, span))
            .map(|old| OSVersion {
                os,
                version: old.0,
                span: old.1,
            })
    }

    pub fn os_cfgs(&self) -> proc_macro2::TokenStream {
        let cfgs = self.versions.keys().map(|key| key.as_nv_cfg());
        quote::quote! {
            #[cfg(any(#(#cfgs),*))]
        }
    }

    pub fn supported_check(&self) -> proc_macro2::TokenStream {
        let checks = self.versions.iter().map(|(&os, &(version, _))| {
            OSVersion {
                os,
                version,
                span: proc_macro2::Span::call_site(),
            }
            .supported_check()
        });
        quote::quote! {
            #(#checks else)* {
                false
            }
        }
    }

    pub fn debug_assert_stmt(&self, func_name: &syn::Ident) -> proc_macro2::TokenStream {
        let checks = self.versions.iter().map(|(&os, &(version, _))| {
            OSVersion {
                os,
                version,
                span: proc_macro2::Span::call_site(),
            }
            .debug_assert_stmt(func_name)
        });
        quote::quote! {
            #(#checks)else*
        }
    }
}

struct OSVersion {
    os:      OS,
    version: Version,
    span:    proc_macro2::Span,
}

impl TryFrom<syn::NestedMeta> for OSVersion {
    type Error = syn::Error;

    fn try_from(nested_meta: syn::NestedMeta) -> parse::Result<Self> {
        match nested_meta {
            syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                let span = nv.span();
                let os = nv.path.try_into()?;
                let version = nv.lit.try_into()?;
                return Ok(Self { os, version, span });
            }
            _ => {}
        };
        Err(syn::Error::new(nested_meta.span(), "`OS_NAME = \"#.#.#\"`"))
    }
}

impl Spanned for OSVersion {
    fn span(&self) -> proc_macro2::Span {
        self.span
    }
}

impl OSVersion {
    fn supported_check(&self) -> proc_macro2::TokenStream {
        let cfg = self.os.as_nv_cfg();
        let Version {
            major,
            minor,
            patch,
        } = self.version;
        quote::quote! {
            if cfg!(#cfg) {
                objc_util::os_atleast!(#major, #minor, #patch)
            }
        }
    }

    fn debug_assert_stmt(&self, func_name: &syn::Ident) -> proc_macro2::TokenStream {
        let os = self.os.as_str();
        let cfg = self.os.as_nv_cfg();
        let Version {
            major,
            minor,
            patch,
        } = self.version;
        quote::quote! {
            if cfg!(#cfg) {
                debug_assert!(
                    objc_util::os_atleast!(#major, #minor, #patch),
                    "`{}` requires `{} {}.{}.{}` but found `{}.{}.{}`",
                    stringify!(#func_name),
                    #os,
                    #major,
                    #minor,
                    #patch,
                    objc_util::OS_VERSION.major,
                    objc_util::OS_VERSION.minor,
                    objc_util::OS_VERSION.patch,
                );
            }
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum OS {
    iOS,
    macOS,
}

impl TryFrom<syn::Path> for OS {
    type Error = syn::Error;

    fn try_from(path: syn::Path) -> parse::Result<Self> {
        match path.get_ident() {
            Some(ident) if ident == "ios" => Ok(OS::iOS),
            Some(ident) if ident == "macos" => Ok(OS::macOS),
            o => Err(syn::Error::new(
                o.span(),
                "Expected one of `ios` or `macos`",
            )),
        }
    }
}

impl OS {
    fn as_str(&self) -> &'static str {
        match self {
            OS::iOS => "ios",
            OS::macOS => "macos",
        }
    }

    fn as_nv_cfg(&self) -> proc_macro2::TokenStream {
        let name = self.as_str();
        quote::quote! {
            target_os = #name
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Version {
    major: i64,
    minor: i64,
    patch: i64,
}

impl TryFrom<syn::Lit> for Version {
    type Error = syn::Error;

    fn try_from(lit: syn::Lit) -> parse::Result<Self> {
        let span = lit.span();
        if let syn::Lit::Str(s) = lit {
            Self::parse(s.value(), s.span())
        } else {
            None
        }.ok_or_else(move || {
            syn::Error::new(
                span,
                "Expected an OS version string (e.g. `\"10.0\"` or `\"12.0.1\"`)",
            )
        })
    }
}

impl Version {
    fn parse(input: String, span: proc_macro2::Span) -> Option<Self> {
        let version: Punctuated::<_, syn::token::Dot> = input.split('.')
            .map(|elem| syn::LitStr::new(elem, span).parse::<syn::LitInt>())
            .collect::<Result<_, _>>()
            .ok()?;
        if version.len() == 0 || version.trailing_punct() || version.len() > 3 {
            return None
        }

        let major = version[0].base10_parse().ok()?;
        let minor = if version.len() > 1 {
            version[1].base10_parse().ok()?
        } else {
            0
        };
        let patch = if version.len() > 2 {
            version[2].base10_parse().ok()?
        } else {
            0
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}
