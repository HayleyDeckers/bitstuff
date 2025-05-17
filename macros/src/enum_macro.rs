use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::str::FromStr;
use syn::{parse::Parser, Expr, ExprLit, ItemEnum, Lit, LitInt, Meta};
use syn::{punctuated::Punctuated, Type};

use super::MultipleErrors;

fn process_variant<'a>(variant: &'a syn::Variant) -> Result<(u128, syn::Ident), syn::Error> {
    let name = &variant.ident;
    let value_expr = match &variant.discriminant {
        Some((_, expr)) => expr,
        None => {
            return Err(syn::Error::new_spanned(
                variant,
                "Each enum variant must have an explicit discriminant (e.g. = 0)",
            ));
        }
    };
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(lit),
        ..
    }) = value_expr
    else {
        return Err(syn::Error::new_spanned(
            value_expr,
            "Only literal integer enum discriminants are supported",
        ));
    };
    let value: u128 = lit
        .base10_parse()
        .map_err(|e| syn::Error::new_spanned(lit, format!("Failed to parse discriminant: {e}")))?;
    Ok((value, name.clone()))
}

fn collect_enum_discriminants<'a>(
    variants: &'a syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> Result<std::collections::BTreeMap<u128, syn::Ident>, syn::Error> {
    let mut all_discriminats = std::collections::BTreeMap::new();
    let mut errs = MultipleErrors::new();
    for variant in variants {
        let (value, ident) = match process_variant(variant) {
            Ok(v) => v,
            Err(e) => {
                errs.combine(e);
                continue;
            }
        };
        if all_discriminats.contains_key(&value) {
            errs.combine(syn::Error::new_spanned(
                variant,
                format!("Duplicate discriminant value: {value}"),
            ));
        } else {
            all_discriminats.insert(value, ident);
        }
    }
    if let Some(e) = errs.into_inner() {
        Err(e)
    } else {
        Ok(all_discriminats)
    }
}

pub fn process(args: TokenStream, input: ItemEnum) -> TokenStream {
    let ItemEnum {
        ident, variants, ..
    } = &input;
    let all_discriminats = match collect_enum_discriminants(variants) {
        Ok(map) => map,
        Err(e) => return e.to_compile_error().into(),
    };
    let (max, _) = match all_discriminats.last_key_value() {
        Some(pair) => pair,
        None => {
            return syn::Error::new_spanned(ident, "Enum must have at least one variant")
                .to_compile_error()
                .into();
        }
    };

    // Parse macro args for bits = N
    let args = match Punctuated::<Meta, syn::Token![,]>::parse_terminated.parse(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };
    let mut explicit_bits = None;
    for arg in &args {
        if let Some(ident) = arg.path().get_ident() {
            if ident == "bits" {
                let name_value = match arg.require_name_value() {
                    Ok(nv) => nv,
                    Err(e) => {
                        return syn::Error::new_spanned(
                            arg,
                            format!("bits arg must be name-value: {e}"),
                        )
                        .to_compile_error()
                        .into()
                    }
                };
                let Expr::Lit(ExprLit {
                    lit: Lit::Int(bits_lit),
                    ..
                }) = &name_value.value
                else {
                    return syn::Error::new_spanned(
                        &name_value.value,
                        "bits arg must be int literal",
                    )
                    .to_compile_error()
                    .into();
                };
                let bits: u32 = match bits_lit.base10_parse() {
                    Ok(b) => b,
                    Err(e) => {
                        return syn::Error::new_spanned(
                            bits_lit,
                            format!("bits must be parseable as a u32: {e}"),
                        )
                        .to_compile_error()
                        .into()
                    }
                };
                explicit_bits = Some(bits);
            }
        }
    }

    let inferred_bits = 128 - max.leading_zeros();
    if let Some(explicit) = explicit_bits {
        if explicit < inferred_bits {
            return syn::Error::new_spanned(
                ident,
                format!(
                    "explicit bits ({}) is less than required to represent all variants ({})",
                    explicit, inferred_bits
                ),
            )
            .to_compile_error()
            .into();
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
                _ => {
                    return syn::Error::new_spanned(ident, "unsupported type")
                        .to_compile_error()
                        .into();
                }
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

    // Generate code
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
