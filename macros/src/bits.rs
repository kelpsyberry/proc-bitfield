use crate::utils::maybe_const_assert;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::borrow::Cow;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Result},
    token::Paren,
    Error, Expr, Lit, Token, Type,
};

mod kw {
    syn::custom_keyword!(above);
    syn::custom_keyword!(below);
}

#[derive(Clone)]
pub struct BitExpr(Expr);

impl BitExpr {
    fn peek(input: ParseStream) -> bool {
        input.peek(Lit) || input.peek(Paren)
    }
}

impl Parse for BitExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            content.parse()?
        } else {
            Expr::Lit(input.parse()?)
        }))
    }
}

impl ToTokens for BitExpr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens)
    }
}

#[derive(Clone)]
pub enum Bits {
    Single(BitExpr),
    SinglePack {
        above_below_span: proc_macro2::Span,
        above: bool,
    },
    Range {
        start: BitExpr,
        end: BitExpr,
    },
    RangeInclusive {
        start: BitExpr,
        end: BitExpr,
    },
    OffsetAndLength {
        start: BitExpr,
        length: BitExpr,
    },
    Pack {
        above_below_span: proc_macro2::Span,
        above: bool,
        length: BitExpr,
    },
    RangeFull,
}

impl Parse for Bits {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(Token![..]) {
            input.parse::<Token![..]>()?;
            Bits::RangeFull
        } else if input.peek(kw::above) || input.peek(kw::below) {
            let (above_below_span, above) = input
                .parse::<kw::above>()
                .map(|a| (a.span, true))
                .or_else(|_| input.parse::<kw::below>().map(|b| (b.span, false)))?;
            if input.parse::<Token![;]>().is_ok() {
                let length = input.parse()?;
                Bits::Pack {
                    above_below_span,
                    above,
                    length,
                }
            } else {
                Bits::SinglePack {
                    above_below_span,
                    above,
                }
            }
        } else if BitExpr::peek(input) {
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
        } else {
            return Err(lookahead.error());
        })
    }
}

impl Bits {
    pub fn into_span(self, last: Option<&BitsSpan>) -> Result<BitsSpan> {
        Ok(match self {
            Bits::Single(bit) => BitsSpan::Single(bit.to_token_stream()),
            Bits::SinglePack {
                above_below_span,
                above,
            } => {
                let (last_start, last_end) =
                    last.and_then(BitsSpan::to_start_end).ok_or_else(|| {
                        Error::new(
                            above_below_span,
                            "cannot use field packing in this position",
                        )
                    })?;
                if above {
                    BitsSpan::Single(last_end.into_owned())
                } else {
                    BitsSpan::Single(quote! { (#last_start) - 1 })
                }
            }
            Bits::Range { start, end } => BitsSpan::Range {
                start: start.to_token_stream(),
                end: end.to_token_stream(),
            },
            Bits::RangeInclusive { start, end } => BitsSpan::Range {
                start: start.to_token_stream(),
                end: quote! { (#end) + 1 },
            },
            Bits::OffsetAndLength { start, length } => {
                let end = quote! { (#start) + (#length) };
                BitsSpan::Range {
                    start: start.to_token_stream(),
                    end,
                }
            }
            Bits::Pack {
                above_below_span,
                above,
                length,
            } => {
                let (last_start, last_end) =
                    last.and_then(BitsSpan::to_start_end).ok_or_else(|| {
                        Error::new(
                            above_below_span,
                            "cannot use field packing in this position",
                        )
                    })?;
                if above {
                    let start = last_end.into_owned();
                    BitsSpan::Range {
                        end: quote! { (#start) + (#length) },
                        start,
                    }
                } else {
                    let end = last_start.into_owned();
                    BitsSpan::Range {
                        start: quote! { (#end) - (#length) },
                        end,
                    }
                }
            }
            Bits::RangeFull => BitsSpan::Full,
        })
    }
}

#[derive(Clone)]
pub enum BitsSpan {
    Single(proc_macro2::TokenStream),
    Range {
        start: proc_macro2::TokenStream,
        end: proc_macro2::TokenStream,
    },
    Full,
}

impl BitsSpan {
    fn to_start_end(
        &'_ self,
    ) -> Option<(
        Cow<'_, proc_macro2::TokenStream>,
        Cow<'_, proc_macro2::TokenStream>,
    )> {
        match self {
            BitsSpan::Single(bit) => Some((Cow::Borrowed(bit), Cow::Owned(quote! { (#bit) + 1 }))),
            BitsSpan::Range { start, end } => Some((Cow::Borrowed(start), Cow::Borrowed(end))),
            _ => None,
        }
    }

    pub fn to_start_end_or_full<'a>(
        &'a self,
        full_bits: &'a proc_macro2::TokenStream,
    ) -> (
        Cow<'a, proc_macro2::TokenStream>,
        Cow<'a, proc_macro2::TokenStream>,
    ) {
        self.to_start_end()
            .unwrap_or_else(|| (Cow::Owned(quote! { 0 }), Cow::Borrowed(full_bits)))
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
        Some(ty) => quote! { ::core::mem::size_of::<#ty>() << 3 },
        None => quote! { ::core::mem::size_of_val(#runtime_value) << 3 },
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
                #storage_ty_assert((#bit) < (#storage_ty_bits));
            }
        }
        BitsSpan::Range { start, end } => {
            quote! {
                ::proc_bitfield::__private::static_assertions::const_assert!((#end) > (#start));
                #storage_ty_assert((#start) < (#storage_ty_bits) && (#end) <= (#storage_ty_bits));
                #field_ty_assert((#end) - (#start) <= (#field_ty_bits));
            }
        }
        BitsSpan::Full => {
            let assert = maybe_const_assert(has_storage_ty && has_field_ty);
            quote! {
                #assert((#storage_ty_bits) <= (#field_ty_bits));
            }
        }
    }
}

struct TyAndAtSign(Type);

impl Parse for TyAndAtSign {
    fn parse(input: ParseStream) -> Result<Self> {
        let ty = input.parse()?;
        input.parse::<Token![@]>()?;
        Ok(TyAndAtSign(ty))
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
            let field_ty = input.parse::<TyAndAtSign>().ok().map(|t| t.0);
            let bits = input.parse()?;
            if !input.is_empty() {
                return Err(input.error("unexpected extra tokens"));
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

    let storage_ty_bits = ty_bits(&storage_ty, quote! { storage_value });
    let field_ty_bits = ty_bits(&field_ty, quote! { &result });
    let bits_span = match bits.into_span(None) {
        Ok(bits_span) => bits_span,
        Err(err) => return err.to_compile_error().into(),
    };
    let asserts = asserts(
        &bits_span,
        storage_ty.is_some(),
        &storage_ty_bits,
        field_ty.is_some(),
        &field_ty_bits,
    );

    let (maybe_uninit_ty, bits_trait) = if let Some(field_ty) = &field_ty {
        (
            quote! { ::core::mem::MaybeUninit::<#field_ty> },
            quote! { ::proc_bitfield::Bits::<#field_ty> },
        )
    } else {
        (
            quote! { ::core::mem::MaybeUninit },
            quote! { ::proc_bitfield::Bits },
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
                let storage_value = &(#storage_value);
                #asserts
                ::proc_bitfield::Bit::bit::<{#bit}>(storage_value)
            }}
        }
        BitsSpan::Range { start, end } => {
            quote! {{
                let storage_value = &(#storage_value);
                let mut result = #maybe_uninit_ty::uninit();
                #asserts
                result = #maybe_uninit_ty::new(
                    #bits_trait::bits::<{#start}, {#end}>(storage_value),
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
                let storage_value = &(#storage_value);
                let mut result = #maybe_uninit_ty::uninit();
                #asserts
                result = #maybe_uninit_ty::new(
                    #bits_trait::bits::<0, {#storage_ty_bits}>(storage_value),
                );
                unsafe { result.assume_init() }
            }}
        }
    }
    .into()
}

struct ModifyBitsArguments {
    storage_value: Expr,
    storage_ty: Option<Type>,
    bits: Bits,
    field_value: Expr,
    field_ty: Option<Type>,
}

impl Parse for ModifyBitsArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let storage_value = input.parse()?;
        let storage_ty = maybe_ty_from_cast_expr(&storage_value);
        input.parse::<Token![,]>()?;
        let pre_field_ty = input.parse::<TyAndAtSign>().ok().map(|t| t.0);
        let bits = input.parse()?;
        input.parse::<Token![=]>()?;
        let field_value = input.parse()?;
        let post_field_ty = maybe_ty_from_cast_expr(&field_value);
        if !input.is_empty() {
            return Err(input.error("unexpected extra tokens"));
        }
        Ok(ModifyBitsArguments {
            storage_value,
            storage_ty,
            bits,
            field_value,
            field_ty: pre_field_ty.or(post_field_ty),
        })
    }
}

pub fn with_bits(input: TokenStream) -> TokenStream {
    let ModifyBitsArguments {
        storage_value,
        storage_ty,
        bits,
        field_value,
        field_ty,
    } = syn::parse_macro_input!(input);

    let storage_ty_bits = ty_bits(&storage_ty, quote! { &storage_value });
    let field_ty_bits = ty_bits(&field_ty, quote! { &field_value });
    let bits_span = match bits.into_span(None) {
        Ok(bits_span) => bits_span,
        Err(err) => return err.to_compile_error().into(),
    };
    let asserts = asserts(
        &bits_span,
        storage_ty.is_some(),
        &storage_ty_bits,
        field_ty.is_some(),
        &field_ty_bits,
    );

    let bits_trait = if let Some(field_ty) = &field_ty {
        quote! { ::proc_bitfield::WithBits::<#field_ty> }
    } else {
        quote! { ::proc_bitfield::WithBits }
    };

    match bits_span {
        BitsSpan::Single(bit) => {
            if field_ty.is_some() {
                return Error::new_spanned(
                    &storage_value,
                    "can't specify a field type for a boolean flag",
                )
                .into_compile_error()
                .into();
            }
            quote! {{
                let storage_value = #storage_value;
                #asserts
                ::proc_bitfield::WithBit::with_bit::<{#bit}>(storage_value, #field_value)
            }}
        }
        BitsSpan::Range { start, end } => {
            quote! {{
                let storage_value = #storage_value;
                let field_value = #field_value;
                #asserts
                #bits_trait::with_bits::<{#start}, {#end}>(storage_value, field_value)
            }}
        }
        BitsSpan::Full => {
            if storage_ty.is_none() {
                return Error::new_spanned(
                    &storage_value,
                    "Input type needs to be specified with `as T` to span the full range",
                )
                .into_compile_error()
                .into();
            }
            quote! {{
                let storage_value = #storage_value;
                let field_value = #field_value;
                #asserts
                #bits_trait::with_bits::<0, {#storage_ty_bits}>(storage_value, field_value)
            }}
        }
    }
    .into()
}

pub fn set_bits(input: TokenStream) -> TokenStream {
    let ModifyBitsArguments {
        storage_value,
        storage_ty,
        bits,
        field_value,
        field_ty,
    } = syn::parse_macro_input!(input);

    let storage_value = match storage_value {
        Expr::Cast(expr_cast) => *expr_cast.expr,
        _ => storage_value,
    };

    let storage_ty_bits = ty_bits(&storage_ty, quote! { storage_value });
    let field_ty_bits = ty_bits(&field_ty, quote! { &field_value });
    let bits_span = match bits.into_span(None) {
        Ok(bits_span) => bits_span,
        Err(err) => return err.to_compile_error().into(),
    };
    let asserts = asserts(
        &bits_span,
        storage_ty.is_some(),
        &storage_ty_bits,
        field_ty.is_some(),
        &field_ty_bits,
    );

    let bits_trait = if let Some(field_ty) = &field_ty {
        quote! { ::proc_bitfield::SetBits::<#field_ty> }
    } else {
        quote! { ::proc_bitfield::SetBits }
    };

    match bits_span {
        BitsSpan::Single(bit) => {
            if field_ty.is_some() {
                return Error::new_spanned(
                    &storage_value,
                    "can't specify a field type for a boolean flag",
                )
                .into_compile_error()
                .into();
            }
            quote! {{
                let storage_value = &mut #storage_value;
                #asserts
                ::proc_bitfield::SetBit::set_bit::<{#bit}>(storage_value, #field_value);
            }}
        }
        BitsSpan::Range { start, end } => {
            quote! {{
                let storage_value = &mut #storage_value;
                let field_value = #field_value;
                #asserts
                #bits_trait::set_bits::<{#start}, {#end}>(storage_value, field_value);
            }}
        }
        BitsSpan::Full => {
            if storage_ty.is_none() {
                return Error::new_spanned(
                    &storage_value,
                    "Input type needs to be specified with `as T` to span the full range",
                )
                .into_compile_error()
                .into();
            }
            quote! {{
                let storage_value = &mut #storage_value;
                let field_value = #field_value;
                #asserts
                #bits_trait::set_bits::<0, {#storage_ty_bits}>(storage_value, field_value);
            }}
        }
    }
    .into()
}
