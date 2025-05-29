use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use std::ops::Range;
use syn::spanned::Spanned;
use syn::{parse::Parser, LitInt};
use syn::{punctuated::Punctuated, Type};
use syn::{
    token::Paren, Attribute, Expr, ExprLit, Fields, FieldsNamed, FieldsUnnamed, Ident, ItemStruct,
    Lit, Meta, Token,
};

enum BitsAttrVal {
    Bit(u32),
    Bits { start: u32, end_exclusive: u32 },
}

impl BitsAttrVal {
    pub fn range(&self) -> Range<u32> {
        match self {
            BitsAttrVal::Bit(bit) => *bit..*bit + 1,
            BitsAttrVal::Bits {
                start,
                end_exclusive,
            } => *start..*end_exclusive,
        }
    }
}

struct FieldAttr {
    bits: BitsAttrVal,
    falliable: bool,
    other: Vec<Attribute>,
}

impl ToTokens for FieldAttr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for attr in &self.other {
            attr.to_tokens(tokens);
        }
    }
}

struct Field {
    ident: Ident,
    ty: Type,
    attr: FieldAttr,
}

impl Field {
    pub fn codegen_with_repr(
        &self,
        repr_type: &syn::Type,
    ) -> Result<proc_macro2::TokenStream, syn::Error> {
        let Field { ident, ty, attr } = self;
        let with_name = format_ident!("with_{}", ident);
        let range = attr.bits.range();
        let start = range.start;
        let end = range.end;
        let n_bits = end - start;
        let mask_val: u128 = (1u128 << n_bits) - 1;
        let mask = Lit::Int(LitInt::new(&format!("{mask_val}"), Span::call_site()));
        let start_lit = Lit::Int(LitInt::new(&format!("{start}"), Span::call_site()));
        let field_bits_path = bitwidth_to_type(ty.span(), n_bits);
        let is_core = matches!(n_bits, 8 | 16 | 32 | 64 | 128);
        let get_bits = if is_core {
            quote! { (#repr_type::from(self.0) >> #start_lit) & #mask }
        } else {
            quote! { <#field_bits_path>::trimmed_new(((self.0 >> #start_lit) & #mask) as _ ) }
        };
        let set_value = quote! { #repr_type::from(<#ty as ::bitstuff::ToBits>::to_bits(value)) };
        if attr.falliable {
            let ret_ty = quote! { ::core::result::Result<#ty, #field_bits_path> };
            Ok(quote! {
                #attr
                #[inline(always)]
                pub fn #ident(&self) -> #ret_ty {
                    let bits = #get_bits;
                    <#ty as ::bitstuff::TryFromBits>::try_from_bits(bits as _)
                }
                #attr
                #[inline(always)]
                pub fn #with_name(mut self, value: #ty) -> Self {
                    let value = #set_value & #mask;
                    self.0 = (((self.0 & !(#mask << #start_lit)) | ((value as #repr_type) << #start_lit)) as _);
                    self
                }
            })
        } else {
            Ok(quote! {
                #attr
                #[inline(always)]
                pub fn #ident(&self) -> #ty {
                    let bits = #get_bits;
                    <#ty as ::bitstuff::FromBits>::from_bits(bits as _)
                }
                #attr
                #[inline(always)]
                pub fn #with_name(mut self, value: #ty) -> Self {
                    let value = #set_value & #mask;
                    self.0 = ((self.0 & !(#mask << #start_lit)) | ((value as #repr_type) << #start_lit)) as _;
                    self
                }
            })
        }
    }
}

impl Field {
    pub fn bits(&self) -> Range<u32> {
        self.attr.bits.range()
    }
}

#[derive(Clone, Copy)]
enum CoreBits {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl From<u32> for CoreBits {
    fn from(bits: u32) -> Self {
        match bits {
            0..8 => CoreBits::U8,
            8..16 => CoreBits::U16,
            16..32 => CoreBits::U32,
            32..64 => CoreBits::U64,
            64..128 => CoreBits::U128,
            _ => panic!("invalid core type"),
        }
    }
}
impl From<CoreBits> for u32 {
    fn from(bits: CoreBits) -> Self {
        match bits {
            CoreBits::U8 => 8,
            CoreBits::U16 => 16,
            CoreBits::U32 => 32,
            CoreBits::U64 => 64,
            CoreBits::U128 => 128,
        }
    }
}

impl CoreBits {
    pub fn to_path(&self) -> syn::TypePath {
        syn::TypePath {
            qself: None,
            path: syn::Path {
                leading_colon: Some(Token![::](Span::call_site())),
                segments: Punctuated::from_iter(vec![
                    syn::PathSegment {
                        ident: Ident::new("core", Span::call_site()),
                        arguments: syn::PathArguments::None,
                    },
                    syn::PathSegment {
                        // ident:  format_ident!("primitive"),
                        ident: Ident::new("primitive", Span::call_site()),
                        arguments: syn::PathArguments::None,
                    },
                    syn::PathSegment {
                        ident: format_ident!("u{}", u32::from(*self)),
                        arguments: syn::PathArguments::None,
                    },
                ]),
            },
        }
    }
}

impl ToTokens for CoreBits {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.to_path().to_tokens(tokens);
    }
}

struct StructAttr {
    bits: Option<u32>,
    repr_bits: Option<CoreBits>,
    original_struct: ItemStruct,
}

impl StructAttr {
    pub fn repr_bits(&self) -> Option<CoreBits> {
        self.repr_bits.or(self.bits.map(CoreBits::from))
    }

    pub fn try_set_bits(&mut self, bits: u32) -> Result<(), syn::Error> {
        if self.bits.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "only one `bits = N` or `uN` argument is allowed",
            ));
        } else {
            self.bits = Some(bits);
            Ok(())
        }
    }

    pub fn try_set_repr_bits(&mut self, repr_bits: CoreBits) -> Result<(), syn::Error> {
        if self.repr_bits.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "only one `repr = uN`  or `uN`argument is allowed",
            ));
        } else {
            self.repr_bits = Some(repr_bits);
            Ok(())
        }
    }
}

struct ParsedInput {
    top_level: StructAttr,
    fields: Vec<Field>,
}

impl ParsedInput {
    pub fn required_repr_bits(&self) -> u32 {
        self.fields.iter().map(|f| f.bits().end).max().unwrap_or(0)
    }

    pub fn min_bits(&self) -> u32 {
        self.top_level.bits.unwrap_or(self.required_repr_bits())
    }

    pub fn repr_bits(&self) -> Result<CoreBits, syn::Error> {
        let required = self.required_repr_bits();
        if let Some(explicit_bits) = self.top_level.repr_bits() {
            let explicit_bits_val = u32::from(explicit_bits);
            if required > explicit_bits_val {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!(
                        "explicit repr bits ({}) is less than required to represent all fields ({}). Highest bit used is {}.",
                        explicit_bits_val,
                        required,
                        required.saturating_sub(1)
                    ),
                ));
            }
            Ok(explicit_bits)
        } else {
            Ok(CoreBits::from(required))
        }
    }
}

fn bitwidth_to_type(span: Span, width: u32) -> syn::Path {
    let is_core_type = matches!(width, 8 | 16 | 32 | 64 | 128);
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

fn parse_field_macro_args(field: &syn::Field) -> Result<FieldAttr, syn::Error> {
    let mut bits_attr = None;
    let mut falliable = false;
    let mut other_attrs = Vec::new();
    for attr in &field.attrs {
        //todo: is this an error?
        let Some(attr_ident) = attr.path().get_ident() else {
            other_attrs.push(attr.clone());
            continue;
        };
        if attr_ident != "bitstuff" {
            other_attrs.push(attr.clone());
            continue;
        }
        // Parse #[bitstuff(bit = N)] or #[bitstuff(bits = N..M)]
        let meta = attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;
        for m in meta {
            if let Some(ident) = m.path().get_ident() {
                match ident.to_string().as_str() {
                    "bit" => {
                        if bits_attr.is_some() {
                            return Err(syn::Error::new_spanned(
                                &m,
                                "only one bit or bits attribute is allowed",
                            ));
                        }
                        if let Meta::NameValue(nv) = m {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Int(litint),
                                ..
                            }) = &nv.value
                            {
                                let bit = litint.base10_parse::<u32>()?;
                                bits_attr = Some(BitsAttrVal::Bit(bit));
                            }
                        }
                    }
                    "bits" => {
                        if bits_attr.is_some() {
                            return Err(syn::Error::new_spanned(
                                &m,
                                "only one bit or bits attribute is allowed",
                            ));
                        }
                        if let Meta::NameValue(nv) = m.clone() {
                            //todo: replace continue's with errors
                            if let Expr::Range(range) = &nv.value {
                                let Some(box_start) = &range.start else {
                                    return Err(syn::Error::new_spanned(
                                        &m,
                                        "open ranges not supported",
                                    ));
                                };
                                let Some(box_end) = &range.end else {
                                    return Err(syn::Error::new_spanned(
                                        &m,
                                        "open ranges not supported",
                                    ));
                                };
                                let Expr::Lit(ExprLit {
                                    lit: Lit::Int(start_lit),
                                    ..
                                }) = &**box_start
                                else {
                                    return Err(syn::Error::new_spanned(
                                        &**box_start,
                                        "expected int literal for start of range",
                                    ));
                                };
                                let Expr::Lit(ExprLit {
                                    lit: Lit::Int(end_lit),
                                    ..
                                }) = &**box_end
                                else {
                                    return Err(syn::Error::new_spanned(
                                        &**box_end,
                                        "expected int literal for end of range",
                                    ));
                                };
                                let start = start_lit.base10_parse::<u32>()?;
                                let mut end = end_lit.base10_parse::<u32>()?;
                                if start >= end {
                                    return Err(syn::Error::new_spanned(
                                        &m,
                                        "end must be greater than start",
                                    ));
                                }
                                if matches!(range.limits, syn::RangeLimits::Closed(_)) {
                                    // end is greater than start and start is u32, so end is atleast 1
                                    end += 1;
                                }
                                bits_attr = Some(BitsAttrVal::Bits {
                                    start,
                                    end_exclusive: end,
                                });
                            }
                        }
                    }
                    "falliable" => {
                        falliable = true;
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &m,
                            format!("unknown bitstuff attribute: {}", ident),
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(
                    &m,
                    "bitstuff attribute must be an ident",
                ));
            }
        }
    }
    let bits_attr = bits_attr.ok_or_else(|| {
        syn::Error::new_spanned(
            field,
            "missing #[bitstuff(bit = N)] or #[bitstuff(bits = N..M)] attribute",
        )
    })?;
    let attr = FieldAttr {
        bits: bits_attr,
        falliable,
        other: other_attrs,
    };
    Ok(attr)
}

fn parse_field(field: &syn::Field) -> Result<Field, syn::Error> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(field, "field must have an ident"))?;
    let ty = field.ty.clone();
    let attr = parse_field_macro_args(field)?;
    Ok(Field { ident, ty, attr })
}

/// Parses a syn::ItemStruct and macro arguments, returns (StructAttr, Vec<Field>)
fn parse_struct(
    input: syn::ItemStruct,
    macro_args: TokenStream,
) -> Result<ParsedInput, syn::Error> {
    let fields = input.fields.clone();
    let struct_attr = parse_struct_macro_args(macro_args, input)?;
    // Parse fields
    let syn::Fields::Named(fields_named) = &fields else {
        return Err(syn::Error::new_spanned(
            &fields,
            "only named fields supported",
        ));
    };
    let fields = fields_named
        .named
        .iter()
        .map(|field| parse_field(field))
        .collect::<Result<Vec<_>, syn::Error>>()?;
    Ok(ParsedInput {
        top_level: struct_attr,
        fields,
    })
}

impl ParsedInput {
    pub fn codegen(self) -> Result<proc_macro2::TokenStream, syn::Error> {
        let repr_type = self.repr_bits()?;
        let repr_type_path = repr_type.to_path();
        let repr_type_ty = syn::Type::Path(repr_type_path.clone());
        let min_bits = self.min_bits();
        let min_bits_is_core = matches!(min_bits, 8 | 16 | 32 | 64 | 128);
        let min_type_ty = bitwidth_to_type(Span::call_site(), min_bits);
        let mut out_struct = self.top_level.original_struct;
        let Fields::Named(FieldsNamed {
            brace_token,
            named: _,
        }) = out_struct.fields
        else {
            return Err(syn::Error::new_spanned(
                out_struct.fields,
                "only named fields supported",
            ));
        };
        out_struct.fields = Fields::Unnamed(FieldsUnnamed {
            unnamed: Punctuated::from_iter(vec![syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Inherited,
                mutability: syn::FieldMutability::None,
                ident: None,
                colon_token: None,
                ty: repr_type_ty.clone(),
            }]),
            paren_token: Paren {
                span: brace_token.span,
            },
        });
        let functions = self
            .fields
            .iter()
            .map(|f| f.codegen_with_repr(&repr_type_ty))
            .collect::<Result<Vec<_>, _>>()?;
        let out_struct_ident = &out_struct.ident;
        // Generate Debug impl using getter methods
        let debug_fields = self.fields.iter().map(|f| {
            let name = f.ident.to_string();
            let ident = &f.ident;
            quote! {
                .field(#name, &self.#ident())
            }
        });
        let trait_impl = if min_bits_is_core {
            quote! {impl ::bitstuff::BitRepr for #out_struct_ident {
                type BitRepr = #min_type_ty;
            }
            impl ::bitstuff::FromBits for #out_struct_ident {
                fn from_bits(bits: #min_type_ty) -> Self {
                    Self(#repr_type_ty::from(bits))
                }
            }
            impl ::bitstuff::ToBits for #out_struct_ident {
                fn to_bits(self) -> #min_type_ty {
                    self.0 as _
                }
            }}
        } else {
            quote! { impl ::bitstuff::BitRepr for #out_struct_ident {
                type BitRepr = #min_type_ty;
            }
            impl ::bitstuff::FromBits for #out_struct_ident {
                fn from_bits(bits: #min_type_ty) -> Self {
                    Self(#repr_type_ty::from(bits))
                }
            }
            impl ::bitstuff::ToBits for #out_struct_ident {
                fn to_bits(self) -> #min_type_ty {
                    #min_type_ty::trimmed_new(self.0 as _)
                }
            }}
        };
        Ok(quote! {
            #out_struct
            impl #out_struct_ident {
                #(#functions)*
            }
            impl ::core::fmt::Debug for #out_struct_ident {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct(stringify!(#out_struct_ident))
                        #(#debug_fields)*
                        .finish()
                }
            }
            #trait_impl

        })
    }
}

// Add back the parse_struct_macro_args function (new version) for struct-level macro argument parsing.
fn parse_struct_macro_args(
    macro_args: TokenStream,
    original_struct: ItemStruct,
) -> Result<StructAttr, syn::Error> {
    let mut struct_attr = StructAttr {
        bits: None,
        repr_bits: None,
        original_struct,
    };
    let args = Punctuated::<Meta, syn::Token![,]>::parse_terminated.parse2(macro_args.into())?;
    for arg in &args {
        let Some(ident) = arg.path().get_ident() else {
            return Err(syn::Error::new_spanned(
                arg,
                "macro argument must be an ident",
            ));
        };
        match ident.to_string().as_str() {
            // bits = N
            "bits" => {
                let Meta::NameValue(nv) = arg else {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "bits must be a name-value pair",
                    ));
                };
                let Expr::Lit(ExprLit {
                    lit: Lit::Int(litint),
                    ..
                }) = &nv.value
                else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "bits must be an int literal",
                    ));
                };
                let bits = litint.base10_parse::<u32>()?;
                struct_attr.try_set_bits(bits)?;
            }
            // repr = uN
            "repr" => {
                let Meta::NameValue(nv) = arg else {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "repr must be a name-value pair",
                    ));
                };
                let Expr::Path(expr_path) = &nv.value else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "repr must be a type path like u32",
                    ));
                };
                let Some(last_seg) = expr_path.path.segments.last() else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "repr must be a type path like u32",
                    ));
                };
                let repr_str = last_seg.ident.to_string();
                struct_attr.try_set_repr_bits(match repr_str.as_str() {
                    "u8" => CoreBits::U8,
                    "u16" => CoreBits::U16,
                    "u32" => CoreBits::U32,
                    "u64" => CoreBits::U64,
                    "u128" => CoreBits::U128,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "repr must be one of u8, u16, u32, u64, u128",
                        ))
                    }
                })?;
            }
            // uN shorthand
            s if s.starts_with("u") => {
                let Ok(bits) = s[1..].parse::<u32>() else {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "expected uN where N is an integer",
                    ));
                };
                struct_attr.try_set_bits(bits)?;
                struct_attr.try_set_repr_bits(match bits {
                    8 => CoreBits::U8,
                    16 => CoreBits::U16,
                    32 => CoreBits::U32,
                    64 => CoreBits::U64,
                    128 => CoreBits::U128,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            arg,
                            "uN shorthand only supports N = 8,16,32,64,128",
                        ))
                    }
                })?;
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    arg,
                    format!("unknown macro argument: {}", ident),
                ));
            }
        }
    }
    Ok(struct_attr)
}

// Macro entrypoint: use new parsing/codegen only
pub fn process(args: TokenStream, input: ItemStruct) -> TokenStream {
    match parse_struct(input, args) {
        Ok(parsed) => match parsed.codegen() {
            Ok(ts) => ts.into(),
            Err(e) => e.into_compile_error().into(),
        },
        Err(e) => e.into_compile_error().into(),
    }
}
