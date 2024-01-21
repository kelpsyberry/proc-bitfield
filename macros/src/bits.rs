use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    token::As,
    Error, Expr, Lit, Token, Type,
};

#[derive(Clone)]
pub enum Bits {
    Single(Lit),
    Range { start: Lit, end: Lit },
    RangeInclusive { start: Lit, end: Lit },
    OffsetAndLength { start: Lit, length: Lit },
    RangeFull,
}

impl Bits {
    pub fn into_span(self) -> BitsSpan {
        match self {
            Bits::Single(bit) => BitsSpan::Single(bit),
            Bits::Range { start, end } => BitsSpan::Range {
                start,
                end: quote! { #end },
            },
            Bits::RangeInclusive { start, end } => BitsSpan::Range {
                start,
                end: quote! { {#end + 1} },
            },
            Bits::OffsetAndLength { start, length } => {
                let end = quote! { {#start + #length} };
                BitsSpan::Range { start, end }
            }
            Bits::RangeFull => BitsSpan::Full,
        }
    }
}

impl Parse for Bits {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.parse::<Token![..]>().is_ok() {
            Bits::RangeFull
        } else {
            let start = input.parse()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![..=]) {
                input.parse::<Token![..=]>()?;
                let end = input.parse()?;
                Bits::RangeInclusive { start, end }
            } else if lookahead.peek(Token![..]) {
                input.parse::<Token![..]>()?;
                let end = input.parse()?;
                Bits::Range { start, end }
            } else if lookahead.peek(Token![;]) {
                input.parse::<Token![;]>()?;
                let length = input.parse()?;
                Bits::OffsetAndLength { start, length }
            } else {
                Bits::Single(start)
            }
        })
    }
}

pub enum BitsSpan {
    Single(Lit),
    Range {
        start: Lit,
        end: proc_macro2::TokenStream,
    },
    Full,
}

fn maybe_const_assert(is_const: bool) -> proc_macro2::TokenStream {
    if is_const {
        quote! { ::proc_bitfield::__private::static_assertions::const_assert! }
    } else {
        quote! { ::core::assert! }
    }
}

fn maybe_ty_from_cast_expr(expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Cast(expr_cast) => {
            (!matches!(*expr_cast.ty, Type::Infer(_))).then(|| (*expr_cast.ty).clone())
        }
        _ => None,
    }
}

fn ty_bits(ty: &Option<Type>, runtime_value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    match ty {
        Some(ty) => quote! { {::core::mem::size_of::<#ty>() << 3} },
        None => quote! { {::core::mem::size_of_val(#runtime_value) << 3} },
    }
}

fn asserts(
    bits_span: &BitsSpan,
    has_storage_ty: bool,
    storage_ty_bits: &proc_macro2::TokenStream,
    has_field_ty: bool,
    field_ty_bits: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let storage_ty_assert = maybe_const_assert(has_storage_ty);
    let field_ty_assert = maybe_const_assert(has_field_ty);
    match bits_span {
        BitsSpan::Single(bit) => {
            quote! {
                #storage_ty_assert(#bit < #storage_ty_bits);
            }
        }
        BitsSpan::Range { start, end } => {
            quote! {
                ::proc_bitfield::__private::static_assertions::const_assert!(#end > #start);
                #storage_ty_assert(#start < #storage_ty_bits && #end <= #storage_ty_bits);
                #field_ty_assert(#end - #start <= #field_ty_bits);
            }
        }
        BitsSpan::Full => {
            let assert = maybe_const_assert(has_storage_ty && has_field_ty);
            quote! {
                #assert(#storage_ty_bits <= #field_ty_bits);
            }
        }
    }
}

pub fn bits(input: TokenStream) -> TokenStream {
    struct Arguments {
        storage_value: Expr,
        storage_ty: Option<Type>,
        bits: Bits,
        field_ty: Option<Type>,
    }

    impl Parse for Arguments {
        fn parse(input: ParseStream) -> Result<Self> {
            let storage_value = input.parse()?;
            let storage_ty = maybe_ty_from_cast_expr(&storage_value);
            input.parse::<Token![,]>()?;
            let bits = input.parse()?;
            let field_ty = if input.parse::<As>().is_ok() {
                Some(input.parse()?)
            } else {
                None
            };
            if !input.is_empty() {
                input.error("unexpected extra tokens");
            }
            Ok(Arguments {
                storage_value,
                storage_ty,
                bits,
                field_ty,
            })
        }
    }

    let Arguments {
        storage_value,
        storage_ty,
        bits,
        field_ty,
    } = syn::parse_macro_input!(input);

    let storage_ty_bits = ty_bits(&storage_ty, quote! { &storage_value });
    let field_ty_bits = ty_bits(&field_ty, quote! { &result });
    let bits_span = bits.into_span();
    let asserts = asserts(
        &bits_span,
        storage_ty.is_some(),
        &storage_ty_bits,
        field_ty.is_some(),
        &field_ty_bits,
    );

    let (maybe_uninit_ty, bit_range_trait) = if let Some(field_ty) = &field_ty {
        (
            quote! { ::core::mem::MaybeUninit::<#field_ty> },
            quote! { ::proc_bitfield::BitRange::<#field_ty> },
        )
    } else {
        (
            quote! { ::core::mem::MaybeUninit },
            quote! { ::proc_bitfield::BitRange },
        )
    };

    match bits_span {
        BitsSpan::Single(bit) => {
            if field_ty.is_some() {
                return Error::new_spanned(
                    &storage_value,
                    "can't specify a field type for a boolean flag",
                )
                .to_compile_error()
                .into();
            }
            quote! {{
                let storage_value = #storage_value;
                #asserts
                ::proc_bitfield::Bit::bit::<#bit>(storage_value)
            }}
        }
        BitsSpan::Range { start, end } => {
            quote! {{
                let storage_value = #storage_value;
                let mut result = #maybe_uninit_ty::uninit();
                #asserts
                result = #maybe_uninit_ty::new(
                    #bit_range_trait::bit_range::<#start, #end>(storage_value),
                );
                unsafe { result.assume_init() }
            }}
        }
        BitsSpan::Full => {
            if storage_ty.is_none() {
                return Error::new_spanned(
                    &storage_value,
                    "input type needs to be specified with `as T` to span the full range",
                )
                .to_compile_error()
                .into();
            }
            quote! {{
                let storage_value = #storage_value;
                let mut result = #maybe_uninit_ty::uninit();
                #asserts
                result = #maybe_uninit_ty::new(
                    #bit_range_trait::bit_range::<0, #storage_ty_bits>(storage_value),
                );
                unsafe { result.assume_init() }
            }}
        }
    }
    .into()
}

fn with_bits_inner(input: TokenStream) -> syn::Result<(proc_macro2::TokenStream, Expr)> {
    struct Arguments {
        storage_value: Expr,
        storage_ty: Option<Type>,
        bits: Bits,
        field_value: Expr,
        field_ty: Option<Type>,
    }

    impl Parse for Arguments {
        fn parse(input: ParseStream) -> Result<Self> {
            let storage_value = input.parse()?;
            let storage_ty = maybe_ty_from_cast_expr(&storage_value);
            input.parse::<Token![,]>()?;
            let bits = input.parse()?;
            input.parse::<Token![=]>()?;
            let field_value = input.parse()?;
            let field_ty = maybe_ty_from_cast_expr(&field_value);
            if !input.is_empty() {
                input.error("unexpected extra tokens");
            }
            Ok(Arguments {
                storage_value,
                storage_ty,
                bits,
                field_value,
                field_ty,
            })
        }
    }

    let Arguments {
        storage_value,
        storage_ty,
        bits,
        field_value,
        field_ty,
    } = syn::parse(input)?;

    let storage_ty_bits = ty_bits(&storage_ty, quote! { &storage_value });
    let field_ty_bits = ty_bits(&field_ty, quote! { &field_value });
    let bits_span = bits.into_span();
    let asserts = asserts(
        &bits_span,
        storage_ty.is_some(),
        &storage_ty_bits,
        field_ty.is_some(),
        &field_ty_bits,
    );

    let bit_range_trait = if let Some(field_ty) = &field_ty {
        quote! { ::proc_bitfield::BitRange::<#field_ty> }
    } else {
        quote! { ::proc_bitfield::BitRange }
    };

    Ok((
        match bits_span {
            BitsSpan::Single(bit) => {
                if field_ty.is_some() {
                    return Err(Error::new_spanned(
                        &storage_value,
                        "can't specify a field type for a boolean flag",
                    ));
                }
                quote! {{
                    let storage_value = #storage_value;
                    #asserts
                    ::proc_bitfield::Bit::set_bit::<#bit>(storage_value, #field_value)
                }}
            }
            BitsSpan::Range { start, end } => {
                quote! {{
                    let storage_value = #storage_value;
                    let field_value = #field_value;
                    #asserts
                    #bit_range_trait::set_bit_range::<#start, #end>(storage_value, field_value)
                }}
            }
            BitsSpan::Full => {
                if storage_ty.is_none() {
                    return Err(Error::new_spanned(
                        &storage_value,
                        "Input type needs to be specified with `as T` to span the full range",
                    ));
                }
                quote! {{
                    let storage_value = #storage_value;
                    let field_value = #field_value;
                    #asserts
                    #bit_range_trait::set_bit_range::<0, #storage_ty_bits>(storage_value, field_value)
                }}
            }
        },
        storage_value,
    ))
}

pub fn with_bits(input: TokenStream) -> TokenStream {
    match with_bits_inner(input) {
        Ok((result, _)) => result.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

pub fn set_bits(input: TokenStream) -> TokenStream {
    match with_bits_inner(input) {
        Ok((result, storage_value)) => {
            let storage_value = match storage_value {
                Expr::Cast(expr_cast) => *expr_cast.expr,
                _ => storage_value,
            };
            quote! { #storage_value = #result; }.into()
        }
        Err(err) => err.into_compile_error().into(),
    }
}
