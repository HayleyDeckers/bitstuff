use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{
    parse_macro_input, token::Paren, Attribute, Expr, ExprLit, Fields, FieldsNamed, FieldsUnnamed,
    Ident, ItemStruct, Lit, LitInt, Meta,
};
use syn::{punctuated::Punctuated, Type};

use super::MultipleErrors;

/// Parses the repr type and returns (bit width, token stream)
fn parse_repr_type(first_arg: &Ident) -> Result<(u32, proc_macro2::TokenStream), syn::Error> {
    let repr_bits = match first_arg.to_string().as_str() {
        "u8" => 8,
        "u16" => 16,
        "u32" => 32,
        "u64" => 64,
        "u128" => 128,
        _ => return Err(syn::Error::new_spanned(first_arg, "unsupported repr type")),
    };
    Ok((repr_bits, first_arg.to_token_stream()))
}

/// Parses the #[bitstuff(...)] attribute and returns (start, end, n_bits, is_falliable)
fn parse_bitstuff_attr(
    attr: &Attribute,
    repr_bits: u32,
) -> Result<(u32, u32, u32, bool), syn::Error> {
    let Meta::List(attr) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "bitstuff attr need to be a list",
        ));
    };
    let args = attr
        .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        .map_err(|e| syn::Error::new_spanned(attr, format!("failed to parse inner args: {e}")))?;
    let mut is_falliable = false;
    let mut start = None;
    let mut end = None;
    for arg in &args {
        if let Some(ident) = arg.path().get_ident() {
            match ident.to_string().as_str() {
                "bit" => {
                    let name_value = arg.require_name_value().map_err(|e| {
                        syn::Error::new_spanned(arg, format!("unexpected arg: {e}"))
                    })?;
                    let Expr::Lit(ExprLit {
                        lit: Lit::Int(bit), ..
                    }) = &name_value.value
                    else {
                        return Err(syn::Error::new_spanned(
                            &name_value.value,
                            "expected a literal int",
                        ));
                    };
                    let bit_val: u32 = bit.base10_parse().map_err(|e| {
                        syn::Error::new_spanned(bit, format!("failed to parse as u32: {e}"))
                    })?;
                    start = Some(bit_val);
                    end = Some(bit_val);
                }
                "bits" => {
                    let name_value = arg.require_name_value().map_err(|e| {
                        syn::Error::new_spanned(arg, format!("unexpected arg: {e}"))
                    })?;
                    let Expr::Range(value) = &name_value.value else {
                        return Err(syn::Error::new_spanned(
                            &name_value.value,
                            "expected a range",
                        ));
                    };
                    let Some(box_start) = &value.start else {
                        return Err(syn::Error::new_spanned(
                            &name_value.value,
                            "open ranges not supported",
                        ));
                    };
                    let Some(box_end) = &value.end else {
                        return Err(syn::Error::new_spanned(
                            &name_value.value,
                            "open ranges not supported",
                        ));
                    };
                    let Expr::Lit(ExprLit {
                        lit: Lit::Int(start_lit),
                        ..
                    }) = &**box_start
                    else {
                        return Err(syn::Error::new_spanned(&**box_start, "expected int start"));
                    };
                    let Expr::Lit(ExprLit {
                        lit: Lit::Int(end_lit),
                        ..
                    }) = &**box_end
                    else {
                        return Err(syn::Error::new_spanned(&**box_end, "expected int end"));
                    };
                    let s: u32 = start_lit.base10_parse().map_err(|e| {
                        syn::Error::new_spanned(start_lit, format!("failed to parse start: {e}"))
                    })?;
                    let mut e: u32 = end_lit.base10_parse().map_err(|e| {
                        syn::Error::new_spanned(end_lit, format!("failed to parse end: {e}"))
                    })?;
                    if matches!(value.limits, syn::RangeLimits::HalfOpen(_)) && e == 0 {
                        return Err(syn::Error::new_spanned(
                            &**box_end,
                            "half open range end cannot be 0",
                        ));
                    }
                    if matches!(value.limits, syn::RangeLimits::HalfOpen(_)) {
                        e -= 1;
                    }
                    start = Some(s);
                    end = Some(e);
                }
                "falliable" => {
                    is_falliable = true;
                }
                _ => {}
            }
        }
    }
    let (start, end) = (
        start.ok_or_else(|| syn::Error::new_spanned(attr, "missing start"))?,
        end.ok_or_else(|| syn::Error::new_spanned(attr, "missing end"))?,
    );
    if end < start {
        return Err(syn::Error::new_spanned(attr, "end not greater than start"));
    }
    if end >= repr_bits {
        return Err(syn::Error::new_spanned(attr, "field out of range"));
    }
    let n_bits = end - start + 1;
    Ok((start, end, n_bits, is_falliable))
}

/// Computes the bitmask for a field
fn bitmask(start: u32, n_bits: u32) -> u128 {
    (1u128.checked_shl(n_bits).unwrap_or(0).wrapping_sub(1)) << start
}

pub fn process_field(
    field: syn::Field,
    set_write_bits: &mut u128,
    repr_type: &proc_macro2::TokenStream,
    repr_bits: u32,
    field_names: &mut Vec<Ident>,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = match &field.ident {
        Some(i) => i,
        None => {
            return Err(syn::Error::new_spanned(&field, "field needs an ident"));
        }
    };
    field_names.push(ident.clone());
    let return_type = &field.ty;
    let mut attr_bitset = None;
    let mut attr_doc = Vec::new();
    let mut attr_other = Vec::new();
    for attr in &field.attrs {
        match attr.path().get_ident().map(Ident::to_string).as_deref() {
            Some("bitstuff") => {
                if attr_bitset.is_some() {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "only one bitstuff attr is supported",
                    ));
                } else {
                    attr_bitset = Some(attr.clone());
                }
            }
            Some("doc") => attr_doc.push(attr.clone()),
            _ => attr_other.push(attr.clone()),
        }
    }
    if attr_bitset.is_none() {
        return Err(syn::Error::new_spanned(
            &field,
            "must have a single bitstuff attribute",
        ));
    }
    let attr_bitset = attr_bitset.unwrap();
    let (start, _end, n_bits, is_falliable) = match parse_bitstuff_attr(&attr_bitset, repr_bits) {
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    };
    let bitmask_val = bitmask(start, n_bits);
    let bitmask_keep_with = Lit::Int(LitInt::new(
        &format!("0b{:b}", bitmask_val),
        Span::call_site(),
    ));
    if *set_write_bits & bitmask_val != 0 {
        return Err(syn::Error::new_spanned(&field, "overlapping bits"));
    }
    *set_write_bits |= bitmask_val;
    let with_function = Ident::new(&format!("with_{ident}"), ident.span());
    let to_bits_type = Type::Verbatim(
        proc_macro2::TokenStream::from_str(&format!(
            "{}{}",
            match n_bits {
                8 | 16 | 32 | 64 | 128 => "::core::primitive::u",
                _ => "::bitstuff::ints::u",
            },
            n_bits
        ))
        .unwrap(),
    );
    let raw_bits_type = if !matches!(n_bits, 8 | 16 | 32 | 64 | 128) {
        proc_macro2::TokenStream::from_str(&format!(
            "::bitstuff::ints::u{n_bits}::trimmed_new((self.0 >> {start}) as _)"
        ))
    } else {
        proc_macro2::TokenStream::from_str(&format!("(self.0 >> {start}) as _"))
    }
    .unwrap();
    Ok(if !is_falliable {
        quote! {
            #(#attr_doc)*
            #[inline(always)]
            #(#attr_other)*
            pub fn #ident(&self) -> #return_type {
                <#return_type as ::bitstuff::FromBits>::from_bits(#raw_bits_type)
            }
            #[inline(always)]
            #(#attr_other)*
            pub fn #with_function(mut self, value: #return_type) -> Self {
                let value : #repr_type = <#return_type as ::bitstuff::ToBits>::to_bits(value).into();
                self.0 = (self.0 & ! #bitmask_keep_with) | (value  << #start);
                self
            }
        }
    } else {
        quote! {
            #(#attr_doc)*
            #[inline(always)]
            #(#attr_other)*
            pub fn #ident(&self) -> ::core::result::Result<#return_type, #to_bits_type> {
                <#return_type as ::bitstuff::TryFromBits>::try_from_bits(#raw_bits_type)
            }
            #[inline(always)]
            #(#attr_other)*
            pub fn #with_function(mut self, value: #return_type) -> Self {
                let value : #repr_type = <#return_type as ::bitstuff::ToBits>::to_bits(value).into();
                self.0 = (self.0 & ! #bitmask_keep_with) | (value  << #start);
                self
            }
        }
    })
}

pub fn process(args: TokenStream, input: ItemStruct) -> TokenStream {
    let ItemStruct {
        attrs,
        vis,
        struct_token,
        ident,
        generics,
        fields,
        semi_token,
    } = input;
    let Fields::Named(FieldsNamed {
        brace_token,
        named: named_fields,
    }) = fields
    else {
        return syn::Error::new_spanned(fields, "only named fields supported")
            .to_compile_error()
            .into();
    };
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let first_arg = match args
        .first()
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "need a repr"))
        .and_then(|m| {
            m.require_path_only().map_err(|e| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("first arg should be a repr: {e}"),
                )
            })
        })
        .and_then(|m| {
            m.get_ident().ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "first arg should be a repr")
            })
        }) {
        Ok(ident) => ident,
        Err(e) => return e.to_compile_error().into(),
    };
    let (repr_bits, repr_type) = match parse_repr_type(first_arg) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut set_write_bits = 0u128;
    let mut functions = Vec::new();
    let mut field_names = Vec::new();
    let mut errors = MultipleErrors::new();
    for field in named_fields {
        match process_field(
            field,
            &mut set_write_bits,
            &repr_type,
            repr_bits,
            &mut field_names,
        ) {
            Ok(v) => functions.push(v),
            Err(e) => errors.combine(e),
        }
    }
    if let Some(e) = errors.into_inner() {
        return e.to_compile_error().into();
    }

    let out_struct = ItemStruct {
        attrs,
        vis,
        struct_token,
        ident: ident.clone(),
        generics,
        semi_token,
        fields: Fields::Unnamed(FieldsUnnamed {
            paren_token: Paren {
                span: brace_token.span,
            },
            unnamed: {
                let mut punct = Punctuated::new();
                punct.push(syn::Field {
                    attrs: Vec::new(),
                    vis: syn::Visibility::Inherited,
                    mutability: syn::FieldMutability::None,
                    ident: None,
                    colon_token: None,
                    ty: syn::Type::Verbatim(repr_type),
                });
                punct
            },
        }),
    };
    TokenStream::from(quote!(
        #out_struct
        impl #ident {
            #(#functions)*
        }
        impl ::core::fmt::Debug for #ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!(#ident))
                #(.field(stringify!(#field_names), &self.#field_names()))*
                    .finish()
            }
        }
    ))
}
