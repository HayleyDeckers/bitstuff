use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::Field;
use syn::Fields;
use syn::{
    parse_macro_input, token::Paren, Attribute, Expr, ExprLit, FieldsNamed, FieldsUnnamed, Ident,
    ItemStruct, Lit, LitInt, Meta,
};
use syn::{punctuated::Punctuated, Type};

pub fn process(args: TokenStream, input: ItemStruct) -> TokenStream {
    // Helper to parse the repr type and get bit width
    fn parse_repr_type(first_arg: &Ident) -> (u32, proc_macro2::TokenStream) {
        let repr_bits = match first_arg.to_string().as_str() {
            "u8" => 8,
            "u16" => 16,
            "u32" => 32,
            "u64" => 64,
            "u128" => 128,
            _ => panic!("unsupported repr type"),
        };
        (repr_bits, first_arg.to_token_stream())
    }

    // Helper to parse bitstuff attribute and return (start, end, n_bits, is_falliable)
    fn parse_bitstuff_attr(attr: &Attribute, repr_bits: u32) -> (u32, u32, u32, bool) {
        let Meta::List(attr) = &attr.meta else {
            panic!("bitstuff attr need to be a list")
        };
        let args = attr
            .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .expect("failed to parse inner args");
        let is_falliable = args.iter().any(|x| x.path().is_ident("falliable"));
        let mut start = None;
        let mut end = None;
        for arg in &args {
            if let Some(ident) = arg.path().get_ident() {
                match ident.to_string().as_str() {
                    "bit" => {
                        let name_value = arg.require_name_value().expect("unexpected arg");
                        let Expr::Lit(ExprLit {
                            lit: Lit::Int(bit), ..
                        }) = &name_value.value
                        else {
                            panic!("expected a literal int")
                        };
                        let bit_val: u32 = bit.base10_parse().expect("failed to parse as u32");
                        start = Some(bit_val);
                        end = Some(bit_val);
                    }
                    "bits" => {
                        let name_value = arg.require_name_value().expect("unexpected arg");
                        let Expr::Range(value) = &name_value.value else {
                            panic!("expected a range")
                        };
                        let Some(box_start) = &value.start else {
                            panic!("open ranges not supported")
                        };
                        let Some(box_end) = &value.end else {
                            panic!("open ranges not supported")
                        };
                        let Expr::Lit(ExprLit {
                            lit: Lit::Int(start_lit),
                            ..
                        }) = &**box_start
                        else {
                            panic!("expected int start")
                        };
                        let Expr::Lit(ExprLit {
                            lit: Lit::Int(end_lit),
                            ..
                        }) = &**box_end
                        else {
                            panic!("expected int end")
                        };
                        let s: u32 = start_lit.base10_parse().expect("failed to parse start");
                        let mut e: u32 = end_lit.base10_parse().expect("failed to parse end");
                        if matches!(value.limits, syn::RangeLimits::HalfOpen(_)) && e == 0 {
                            panic!("half open range end cannot be 0");
                        }
                        if matches!(value.limits, syn::RangeLimits::HalfOpen(_)) {
                            e -= 1;
                        }
                        start = Some(s);
                        end = Some(e);
                    }
                    "falliable" => { /* already handled */ }
                    _ => {}
                }
            }
        }
        let (start, end) = (start.expect("missing start"), end.expect("missing end"));
        if end < start {
            panic!("end not greater than start")
        }
        if end >= repr_bits {
            panic!("field out of range")
        }

        let n_bits = end - start + 1;
        (start, /*_end*/ end, n_bits, is_falliable)
    }

    // Helper to compute bitmask
    fn bitmask(start: u32, n_bits: u32) -> u128 {
        (1u128.checked_shl(n_bits).unwrap_or(0).wrapping_sub(1)) << start
    }

    // --- Main logic ---
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
    let (repr_bits, repr_type) = parse_repr_type(first_arg);

    let mut set_write_bits = 0u128;
    let mut functions = Vec::new();
    let mut field_names = Vec::new();
    for field in named_fields {
        let Some(ident) = field.ident else {
            panic!("field needs and ident");
        };
        field_names.push(ident.clone());
        let return_type = field.ty;
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
                Some("doc") => attr_doc.push(attr),
                _ => attr_other.push(attr),
            }
        }
        let attr_bitset = attr_bitset.expect("must have a single bitstuff attribute");
        let (start, _end, n_bits, is_falliable) = parse_bitstuff_attr(&attr_bitset, repr_bits);
        let bitmask_val = bitmask(start, n_bits);
        let bitmask_keep_with = Lit::Int(LitInt::new(
            &format!("0b{:b}", bitmask_val),
            Span::call_site(),
        ));
        if set_write_bits & bitmask_val != 0 {
            panic!("overlapping bits");
        }
        set_write_bits |= bitmask_val;
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
        functions.push(if !is_falliable {
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
        });
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
