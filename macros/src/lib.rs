use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::Field;
use syn::Fields;
use syn::{
    parse_macro_input, token::Paren, Attribute, Expr, ExprLit, FieldsNamed, FieldsUnnamed, Ident,
    ItemEnum, ItemStruct, Lit, LitInt, Meta, RangeLimits,
};
use syn::{punctuated::Punctuated, Type};

#[proc_macro_attribute]
pub fn stuff(args: TokenStream, input: TokenStream) -> TokenStream {
    //applies to either a struct, or an enum
    // if enum, we generate a field that can be put in the struct.
    // for the struct, the valid field are the primitive unsigned ints
    // a bool (if 1 bit), or an enum
    if let Ok(input) = syn::parse(input.clone()) {
        process_itemstruct(args, input)
    } else if let Ok(input) = syn::parse(input) {
        process_itemenum(args, input)
    } else {
        panic!("not enum or struct")
    }
}

fn process_itemenum(_args: TokenStream, input: ItemEnum) -> TokenStream {
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

    let required_bits = 128 - max.leading_zeros();
    let is_complete = all_discriminats.len() == 1 << required_bits;

    let is_core_type = matches!(required_bits, 8 | 16 | 32 | 64 | 128);

    let to_bits_type = Type::Verbatim(
        proc_macro2::TokenStream::from_str(&format!(
            "{}{required_bits}",
            if is_core_type {
                "::core::primitive::u"
            } else {
                "::bitstuff::ints::u"
            }
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
                // let ident = format!("Self::{ident}"));
                let value = LitInt::new(&format!("{value}"), Span::call_site());
                quote! {Self::#ident => #value,}
            })
            .collect::<proc_macro2::TokenStream>();
        let from_bits_match_body = all_discriminats
            .iter()
            .map(|(value, ident)| {
                // let ident = format!("Self::{ident}"));
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

fn process_itemstruct(args: TokenStream, input: ItemStruct) -> TokenStream {
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
        panic!("only named fields supported")
    };
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let first_arg = args
        .first()
        .expect("need a repr")
        .require_path_only()
        .expect("first arg should be a repr")
        .get_ident()
        .expect("first arg repr again");
    let repr_bits = match first_arg.to_string().as_str() {
        "u8" => 8,
        "u16" => 16,
        "u32" => 32,
        "u64" => 64,
        "u128" => 128,
        _ => panic!("unsupported repr type"),
    };
    let repr_type = first_arg.to_token_stream();

    let mut set_write_bits = 0u128;
    let mut functions = Vec::new();
    let mut field_names = Vec::new();
    for field in named_fields {
        let Some(ident) = field.ident else {
            panic!("field needs and ident");
        };
        field_names.push(ident.clone());
        let return_type = field.ty;
        // should have 1 bitstuff attr, n doc attr, nothing else? what about allow non_snake_case etc
        let mut attr_bitset = None;
        let mut attr_doc = Vec::new();
        let mut attr_other = Vec::new();
        for attr in field.attrs {
            match attr.path().get_ident().map(Ident::to_string).as_deref() {
                Some("bitstuff") => {
                    if attr_bitset.replace(attr).is_some() {
                        panic!("only one bitstuff attr is supported")
                    }
                }
                Some("doc") => {
                    attr_doc.push(attr);
                }
                _ => attr_other.push(attr),
            }
        }
        let Some(attr_bitset) = attr_bitset else {
            panic!("must have a single bitstuff attribute")
        };
        let Attribute {
            pound_token: _,
            style: _,
            bracket_token: _,
            meta: Meta::List(attr),
        } = attr_bitset
        else {
            panic!("bitstuff attr need to be a list")
        };
        let args = attr
            .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .expect("failed to parse inner args");
        let is_falliable = args
            .iter()
            .find(|x| x.path().is_ident("falliable"))
            .is_some();
        for arg in args {
            match arg
                .path()
                .get_ident()
                .expect("no ident on name_value")
                .to_string()
                .as_str()
            {
                "falliable" => {
                    // already handled above
                    arg.require_path_only()
                        .expect("fallible arg should be a path");
                }
                bits @ ("bits" | "bit") => {
                    let name_value = arg.require_name_value().expect("unexpected arg").clone();
                    //todo: check that only one of bit or bits is set
                    let (start, end, is_halfopen) = if bits == "bit" {
                        let Expr::Lit(ExprLit {
                            attrs: _,
                            lit: Lit::Int(bit),
                        }) = name_value.value
                        else {
                            panic!("expected a literal int")
                        };
                        (bit.clone(), bit, false)
                    } else {
                        let Expr::Range(value) = name_value.value else {
                            panic!("expected a range")
                        };
                        let is_halfopen = match value.limits {
                            RangeLimits::Closed(_) => false,
                            RangeLimits::HalfOpen(_) => true,
                        };
                        let Some(box_start) = value.start else {
                            panic!("open ranges not supported")
                        };
                        let Some(box_end) = value.end else {
                            panic!("open ranges not supported")
                        };
                        let (
                            Expr::Lit(ExprLit {
                                attrs: _,
                                lit: Lit::Int(start),
                            }),
                            Expr::Lit(ExprLit {
                                attrs: _,
                                lit: Lit::Int(end),
                            }),
                        ) = (*box_start, *box_end)
                        else {
                            panic!("expected range of ints")
                        };
                        (start, end, is_halfopen)
                    };
                    let start: u32 = start.base10_parse().expect("failed to parse as u32");
                    let mut end: u32 = end.base10_parse().expect("failed to parse as u32");
                    if is_halfopen {
                        if end == 0 {
                            panic!("half open range end cannot be 0")
                        }
                        end -= 1;
                    }
                    if end < start {
                        panic!("end not greater than start")
                    }

                    if end >= repr_bits {
                        panic!("field out of range")
                    }
                    let n_bits = end - start + 1;
                    let bitmask = (1u128.checked_shl(n_bits).unwrap_or(0).wrapping_sub(1)) << start;
                    let bitmask_keep_with =
                        Lit::Int(LitInt::new(&format!("0b{:b}", bitmask), Span::call_site()));
                    if set_write_bits & bitmask != 0 {
                        panic!("overlapping bits");
                    }
                    set_write_bits |= bitmask;

                    let with_function = Ident::new(&format!("with_{ident}"), ident.span());

                    let to_bits_type = Type::Verbatim(
                        proc_macro2::TokenStream::from_str(&format!(
                            "{}{n_bits}",
                            match n_bits {
                                8 | 16 | 32 | 64 | 128 => "::core::primitive::u",
                                _ => "::bitstuff::ints::u",
                            }
                        ))
                        .unwrap(),
                    );
                    // make a function call for the trimmed_new function

                    let raw_bits_type = if !matches!(n_bits, 8 | 16 | 32 | 64 | 128) {
                        proc_macro2::TokenStream::from_str(&format!(
                            "::bitstuff::ints::u{n_bits}::trimmed_new((self.0 >> {start}) as _)",
                        ))
                    } else {
                        proc_macro2::TokenStream::from_str(&format!("(self.0 >> {start}) as _",))
                    }
                    .unwrap();
                    functions.push(if !is_falliable{ quote! {
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
                    }} else {
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
                                //todo: might be able to use FromBits here instead of From to keep the set of traits used smaller
                                let value : #repr_type = <#return_type as ::bitstuff::ToBits>::to_bits(value).into();
                                self.0 = (self.0 & ! #bitmask_keep_with) | (value  << #start);
                                self
                            }
                        }
                    })
                }
                _ => panic!("invalid arg name"),
            }
        }
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
                punct.push(Field {
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
    // Hand the resulting function body back to the compiler.
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
