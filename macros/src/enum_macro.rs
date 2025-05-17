use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, Expr, ExprLit, ItemEnum, Lit, LitInt, Meta};
use syn::{punctuated::Punctuated, Type};

pub fn process(args: TokenStream, input: ItemEnum) -> TokenStream {
    let ItemEnum {
        ident, variants, ..
    } = input.clone();
    let mut all_discriminats = std::collections::BTreeMap::new();
    for variant in variants {
        let name = variant.ident;
        let value_expr = variant.discriminant.expect("needs explicit discriminant").1;
        let Expr::Lit(ExprLit {
            attrs: _,
            lit: Lit::Int(lit),
        }) = value_expr
        else {
            panic!("only literal enum discriminat supported")
        };
        let value: u128 = lit.base10_parse().unwrap();

        if all_discriminats.insert(value, name).is_some() {
            panic!("duplicate discriminant");
        }
    }
    let (max, _) = all_discriminats
        .last_key_value()
        .expect("needs atleast one entry");

    // Parse macro args for bits = N
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let mut explicit_bits = None;
    for arg in &args {
        if let Some(ident) = arg.path().get_ident() {
            if ident == "bits" {
                let name_value = arg
                    .require_name_value()
                    .expect("bits arg must be name-value");
                let Expr::Lit(ExprLit {
                    lit: Lit::Int(bits_lit),
                    ..
                }) = &name_value.value
                else {
                    panic!("bits arg must be int literal")
                };
                let bits: u32 = bits_lit.base10_parse().expect("bits must be u32");
                explicit_bits = Some(bits);
            }
        }
    }

    let inferred_bits = 128 - max.leading_zeros();
    if let Some(explicit) = explicit_bits {
        if explicit < inferred_bits {
            panic!(
                "explicit bits ({}) is less than required to represent all variants ({})",
                explicit, inferred_bits
            );
        }
    }
    let required_bits = explicit_bits.unwrap_or(inferred_bits);
    let is_complete = all_discriminats.len() == 1 << required_bits;
    let is_core_type = matches!(required_bits, 8 | 16 | 32 | 64 | 128);

    let to_bits_type = Type::Verbatim(
        proc_macro2::TokenStream::from_str(&format!(
            "{}{}",
            if is_core_type {
                "::core::primitive::u"
            } else {
                "::bitstuff::ints::u"
            },
            required_bits
        ))
        .unwrap(),
    );
    let repr_type = Type::Verbatim(
        proc_macro2::TokenStream::from_str(&format!(
            "u{}",
            match required_bits {
                x if x <= 8 => 8,
                x if x <= 16 => 16,
                x if x <= 32 => 32,
                x if x <= 64 => 64,
                x if x <= 128 => 128,
                _ => panic!("unsupported type"),
            }
        ))
        .unwrap(),
    );
    let to_bits_convert = if is_core_type {
        proc_macro2::TokenStream::new()
    } else {
        quote! {
            #to_bits_type::trimmed_new
        }
    };
    if is_complete {
        let to_bits_match_body = all_discriminats
            .iter()
            .map(|(value, ident)| {
                let value = LitInt::new(&format!("{value}"), Span::call_site());
                quote! {Self::#ident => #value,}
            })
            .collect::<proc_macro2::TokenStream>();
        let from_bits_match_body = all_discriminats
            .iter()
            .map(|(value, ident)| {
                let value = LitInt::new(&format!("{value}"), Span::call_site());
                quote! { #value => Self::#ident,}
            })
            .collect::<proc_macro2::TokenStream>();

        TokenStream::from(quote! { #input

            impl ::bitstuff::BitRepr for #ident {
                type BitRepr = #to_bits_type;
            }
            impl ::bitstuff::FromBits for #ident {
                fn from_bits(value: #to_bits_type) -> Self {
                    match #repr_type::from(value) {
                        #from_bits_match_body
                        _ => unreachable!(),
                    }
                }
            }

            impl ::bitstuff::ToBits for #ident {
                fn to_bits(self) -> #to_bits_type {
                    #to_bits_convert(match self {
                        #to_bits_match_body
                    })
                }
        }
        })
    } else {
        let to_bits_match_body = all_discriminats
            .iter()
            .map(|(value, ident)| {
                let value = LitInt::new(&format!("{value}"), Span::call_site());
                quote! {Self::#ident => #value,}
            })
            .collect::<proc_macro2::TokenStream>();
        let from_bits_match_body = all_discriminats
            .iter()
            .map(|(value, ident)| {
                let value = LitInt::new(&format!("{value}"), Span::call_site());
                quote! { #value => Ok(Self::#ident),}
            })
            .collect::<proc_macro2::TokenStream>();
        TokenStream::from(quote! { #input
            impl ::bitstuff::TryFromBits for #ident {
                fn try_from_bits(value: #to_bits_type) -> ::core::result::Result<Self,#to_bits_type> {
                    match #repr_type::from(value) {
                        #from_bits_match_body
                        _ => Err(value),
                    }
                }
            }

            impl ::bitstuff::ToBits for #ident {
                fn to_bits(self) -> #to_bits_type {
                    #to_bits_convert(match self {
                        #to_bits_match_body
                        })
                }
            }
            impl ::bitstuff::BitRepr for #ident {
                type BitRepr = #to_bits_type;
            }
        })
    }
}
