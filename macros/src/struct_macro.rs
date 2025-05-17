use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::str::FromStr;
use syn::{
    parse_macro_input, spanned::Spanned, token::Paren, Attribute, Expr, ExprLit, Fields,
    FieldsNamed, FieldsUnnamed, Ident, ItemStruct, Lit, LitInt, Meta, Token,
};
use syn::{punctuated::Punctuated, Type};

use super::MultipleErrors;

/// Parses the repr type and returns (bit width, token stream)
fn parse_repr_bits(first_arg: &Ident) -> Result<u32, syn::Error> {
    let repr_bits = first_arg
        .to_string()
        .strip_prefix("u")
        .ok_or_else(|| syn::Error::new_spanned(first_arg, "expected uN where 0 < N <= 128"))?
        .parse::<u32>()
        .map_err(|e| {
            syn::Error::new_spanned(first_arg, format!("expected uN where 0 < N <= 128: {e}"))
        })?;
    if repr_bits > 128 || repr_bits == 0 {
        return Err(syn::Error::new_spanned(
            first_arg,
            "expected uN where  0 < N <= 128",
        ));
    }
    Ok(repr_bits)
}

fn to_nearest_core_type(width: u32) -> u32 {
    if width <= 8 {
        8
    } else if width <= 16 {
        16
    } else if width <= 32 {
        32
    } else if width <= 64 {
        64
    } else if width <= 128 {
        128
    } else {
        panic!("bitwidth {width} is not supported");
    }
}

fn bitwidth_to_type(span: Span, width: u32) -> syn::Path {
    let is_core_type = is_core_type(width);
    syn::Path {
        leading_colon: Some(Token![::](span)),
        segments: Punctuated::from_iter(vec![
            syn::PathSegment {
                ident: if is_core_type {
                    format_ident!("core")
                } else {
                    format_ident!("bitstuff")
                },
                arguments: syn::PathArguments::None,
            },
            syn::PathSegment {
                ident: if is_core_type {
                    format_ident!("primitive")
                } else {
                    format_ident!("ints")
                },
                arguments: syn::PathArguments::None,
            },
            syn::PathSegment {
                ident: format_ident!("u{width}"),
                arguments: syn::PathArguments::None,
            },
        ]),
    }
}

/// Parses the #[bitstuff(...)] attribute and returns (start, end, n_bits, is_falliable)
fn parse_bitstuff_attr(
    attr: &Attribute,
    // repr_bits: u32,
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
    // if end >= repr_bits {
    //     return Err(syn::Error::new_spanned(attr, "field out of range"));
    // }
    let n_bits = end - start + 1;
    Ok((start, end, n_bits, is_falliable))
}

/// Computes the bitmask for a field
fn bitmask(start: u32, n_bits: u32) -> u128 {
    (1u128.checked_shl(n_bits).unwrap_or(0).wrapping_sub(1)) << start
}

fn is_core_type(n_bits: u32) -> bool {
    matches!(n_bits, 8 | 16 | 32 | 64 | 128)
}

/// Struct holding parsed field info for codegen
struct ParsedField {
    ident: Ident,
    ty: Type,
    start: u32,
    end: u32,
    n_bits: u32,
    is_falliable: bool,
    attr_doc: Vec<Attribute>,
    attr_other: Vec<Attribute>,
    inline_attr: Option<Attribute>,
    span: proc_macro2::Span,
}

/// Parse all struct fields, returning parsed fields and inferred min bit size
fn parse_struct_fields(
    fields: Punctuated<syn::Field, syn::token::Comma>,
    explicit_repr_bits: Option<u32>,
) -> Result<(Vec<ParsedField>, u32), syn::Error> {
    let mut set_write_bits = 0u128;
    let mut parsed = Vec::new();
    let mut max_bit = 0u32;
    let mut errors = MultipleErrors::new();
    for field in fields {
        let ident = match &field.ident {
            Some(i) => i.clone(),
            None => {
                errors.combine(syn::Error::new_spanned(&field, "field needs an ident"));
                continue;
            }
        };
        let mut attr_bitset = None;
        let mut attr_doc = Vec::new();
        let mut attr_other = Vec::new();
        let mut inline_attr = None;
        for attr in &field.attrs {
            match attr.path().get_ident().map(Ident::to_string).as_deref() {
                Some("bitstuff") => {
                    if attr_bitset.is_some() {
                        errors.combine(syn::Error::new_spanned(
                            attr,
                            "only one bitstuff attr is supported",
                        ));
                    } else {
                        attr_bitset = Some(attr.clone());
                    }
                }
                Some("doc") => attr_doc.push(attr.clone()),
                Some("inline") => inline_attr = Some(attr.clone()),
                _ => attr_other.push(attr.clone()),
            }
        }
        if attr_bitset.is_none() {
            errors.combine(syn::Error::new_spanned(
                &field,
                "must have a single bitstuff attribute",
            ));
            continue;
        }
        let attr_bitset = attr_bitset.unwrap();
        let (start, end, n_bits, is_falliable) = match parse_bitstuff_attr(&attr_bitset) {
            Ok(v) => v,
            Err(e) => {
                errors.combine(e);
                continue;
            }
        };
        let bitmask_val = bitmask(start, n_bits);
        if set_write_bits & bitmask_val != 0 {
            errors.combine(syn::Error::new_spanned(&field, "overlapping bits"));
            continue;
        }
        set_write_bits |= bitmask_val;
        if end > max_bit {
            max_bit = end;
        }
        let span = field.span();
        parsed.push(ParsedField {
            ident,
            ty: field.ty,
            start,
            end,
            n_bits,
            is_falliable,
            attr_doc,
            attr_other,
            inline_attr,
            span,
        });
    }
    if let Some(e) = errors.into_inner() {
        return Err(e);
    }
    // Inferred repr_bits is max_bit+1 (arbitrary up to 128)
    let inferred = max_bit + 1;
    if inferred > 128 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("bitfield needs {} bits, max supported is 128", inferred),
        ));
    }
    if let Some(explicit) = explicit_repr_bits {
        if explicit < inferred {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "explicit repr bits ({explicit}) is less than required to represent all fields ({inferred})"
                ),
            ));
        }
    }
    Ok((parsed, inferred))
}

/// Generate code for all parsed fields
fn codegen_struct_fields(
    fields: &[ParsedField],
    repr_type: &syn::Path,
) -> (Vec<proc_macro2::TokenStream>, Vec<Ident>) {
    let mut out = Vec::new();
    let mut field_names = Vec::new();
    for f in fields {
        let ParsedField {
            ident,
            ty,
            start,
            n_bits,
            is_falliable,
            attr_doc,
            attr_other,
            inline_attr,
            span,
            ..
        } = f;
        field_names.push(ident.clone());
        let with_function = Ident::new(&format!("with_{}", ident), ident.span());
        let bitmask_val = bitmask(*start, *n_bits);
        let bitmask_keep_with = LitInt::new(&format!("0b{:b}", bitmask_val), Span::call_site());
        let inline_attr_tokens = inline_attr
            .as_ref()
            .map(|a| a.to_token_stream())
            .unwrap_or_else(|| quote_spanned! {*span=> #[inline(always)] });

        let to_bits_type = Type::Verbatim(
            proc_macro2::TokenStream::from_str(&format!(
                "{}{}",
                if is_core_type(*n_bits) {
                    "::core::primitive::u"
                } else {
                    "::bitstuff::ints::u"
                },
                n_bits
            ))
            .expect("failed to generate to_bits_type"),
        );
        let raw_bits_type = if is_core_type(*n_bits) {
            proc_macro2::TokenStream::from_str(&format!(
                "(self.0 >> {start}) as ::core::primitive::u{n_bits}"
            ))
            .expect("failed to generate raw_bits_type for core type")
        } else {
            proc_macro2::TokenStream::from_str(&format!(
                "::bitstuff::ints::u{n_bits}::trimmed_new((self.0 >> {start}) as _)"
            ))
            .expect("failed to generate raw_bits_type for non core type")
        };
        let method = if !is_falliable {
            quote_spanned! {*span=>
                #(#attr_doc)*
                #inline_attr_tokens
                #(#attr_other)*
                pub fn #ident(&self) -> #ty {
                    <#ty as ::bitstuff::FromBits>::from_bits(#raw_bits_type)
                }
                #inline_attr_tokens
                #(#attr_other)*
                pub fn #with_function(mut self, value: #ty) -> Self {
                    let value : #repr_type = <#ty as ::bitstuff::ToBits>::to_bits(value).into();
                    self.0 = (self.0 & ! #bitmask_keep_with) | (value  << #start);
                    self
                }
            }
        } else {
            quote_spanned! {*span=>
                #(#attr_doc)*
                #inline_attr_tokens
                #(#attr_other)*
                pub fn #ident(&self) -> ::core::result::Result<#ty, #to_bits_type> {
                    <#ty as ::bitstuff::TryFromBits>::try_from_bits(#raw_bits_type)
                }
                #inline_attr_tokens
                #(#attr_other)*
                pub fn #with_function(mut self, value: #ty) -> Self {
                    let value : #repr_type = <#ty as ::bitstuff::ToBits>::to_bits(value).into();
                    self.0 = (self.0 & ! #bitmask_keep_with) | (value  << #start);
                    self
                }
            }
        };
        out.push(method);
    }
    (out, field_names)
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
    // Extract explicit repr ident if present
    let explicit_repr = args.first().and_then(|m| match m {
        syn::Meta::Path(path) => path.get_ident().cloned(),
        _ => None,
    });
    let explicit_repr_bits = match explicit_repr.as_ref().map(|ident| parse_repr_bits(ident)) {
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => return e.to_compile_error().into(),
        None => None,
    };
    let (parsed_fields, inferred_bits) = match parse_struct_fields(named_fields, explicit_repr_bits)
    {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let bitwidth = explicit_repr_bits.unwrap_or(inferred_bits);
    let repr_bitwidth = to_nearest_core_type(bitwidth);
    let repr_type = bitwidth_to_type(Span::call_site(), repr_bitwidth);
    let bitwidth_ty = bitwidth_to_type(Span::call_site(), bitwidth);

    // Use explicit repr if present, else infer

    let (functions, field_names) = codegen_struct_fields(&parsed_fields, &repr_type);
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
                    ty: syn::Type::Verbatim(repr_type.to_token_stream()),
                });
                punct
            },
        }),
    };

    let trait_impl = if bitwidth != repr_bitwidth {
        quote!(
        impl ::bitstuff::BitRepr for #ident {
            type BitRepr = #bitwidth_ty;
        }
        impl ::bitstuff::ToBits for #ident {
            fn to_bits(self) -> #bitwidth_ty {
                #bitwidth_ty::trimmed_new(self.0)
            }
        }
        impl ::bitstuff::FromBits for #ident {
            fn from_bits(bits: #bitwidth_ty) -> Self {
                Self(bits.into())
            }
        })
    } else {
        quote!(
        impl ::bitstuff::BitRepr for #ident {
            type BitRepr = #bitwidth_ty;
        }
        impl ::bitstuff::ToBits for #ident {
            fn to_bits(self) -> #bitwidth_ty {
                self.0
            }
        }
        impl ::bitstuff::FromBits for #ident {
            fn from_bits(bits: #bitwidth_ty) -> Self {
                Self(bits)
            }
        })
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
        #trait_impl
    ))
}
