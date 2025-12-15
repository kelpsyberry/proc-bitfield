#[cfg(feature = "gce")]
use crate::utils::{add_const_bounds, const_param, doc_hidden, min, sized_pred, type_param};
use crate::{
    bits::{Bits, BitsSpan},
    utils::{
        maybe_const_assert, parse_braces, parse_brackets, parse_parens, parse_terminated,
        MaybeRepeat,
    },
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
#[cfg(feature = "gce")]
use std::borrow::Cow;
use syn::{
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Error, Expr, ExprParen, ExprPath, Generics, Ident, Token, Type, TypeParam,
    Visibility, WhereClause,
};
#[cfg(feature = "gce")]
use syn::{
    Lifetime, LifetimeParam, Path, PathArguments, PathSegment, TraitBound, TraitBoundModifier,
    TypePath, TypeReference,
};

mod kw {
    syn::custom_keyword!(nested);

    syn::custom_keyword!(get);
    syn::custom_keyword!(set);

    syn::custom_keyword!(get_fn);
    syn::custom_keyword!(set_fn);

    syn::custom_keyword!(unsafe_get);
    syn::custom_keyword!(unsafe_set);
    syn::custom_keyword!(unsafe_both);

    syn::custom_keyword!(unsafe_get_fn);
    syn::custom_keyword!(unsafe_set_fn);

    syn::custom_keyword!(try_get);
    syn::custom_keyword!(try_set);
    syn::custom_keyword!(try_both);

    syn::custom_keyword!(try_get_fn);
    syn::custom_keyword!(try_set_fn);

    syn::custom_keyword!(unwrap_get);
    syn::custom_keyword!(unwrap_set);
    syn::custom_keyword!(unwrap_both);
    syn::custom_keyword!(unwrap);

    syn::custom_keyword!(unwrap_get_fn);
    syn::custom_keyword!(unwrap_set_fn);

    syn::custom_keyword!(read_only);
    syn::custom_keyword!(ro);
    syn::custom_keyword!(write_only);
    syn::custom_keyword!(wo);

    syn::custom_keyword!(no_const);

    syn::custom_keyword!(Debug);
    syn::custom_keyword!(FromStorage);
    syn::custom_keyword!(IntoStorage);
    syn::custom_keyword!(DerefStorage);
}

fn parse_accessor_fn(input: ParseStream) -> Result<Expr> {
    input
        .parse::<ExprParen>()
        .map(Expr::Paren)
        .or_else(|_| input.parse::<ExprPath>().map(Expr::Path))
}

enum AccessorKind {
    Default,
    ConvTy(Type),
    ConvFn {
        fn_: Expr,
        ty: Type,
    },
    UnsafeConvTy {
        ty: Type,
        has_safe_accessor: bool,
    },
    UnsafeConvFn {
        fn_: Expr,
        ty: Type,
        has_safe_accessor: bool,
    },
    TryConvTy(Type),
    TryGetFn {
        fn_: Expr,
        result_ty: Type,
    },
    TrySetFn {
        fn_: Expr,
        input_ty: Type,
        result_ty: Type,
    },
    UnwrapConvTy(Type),
    UnwrapConvFn {
        fn_: Expr,
        ty: Type,
    },
    Disabled,
}

impl AccessorKind {
    fn is_unsafe(&self) -> bool {
        matches!(
            self,
            AccessorKind::UnsafeConvTy {
                has_safe_accessor: false,
                ..
            } | AccessorKind::UnsafeConvFn {
                has_safe_accessor: false,
                ..
            }
        )
    }
}

struct SingleField {
    get_kind: AccessorKind,
    set_kind: AccessorKind,
}

struct NestedField {
    is_readable: bool,
    is_writable: bool,
}

enum FieldContent {
    Single(SingleField),
    Nested(NestedField),
}

struct Field {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    bits: Bits,
    ty: Type,
    content: FieldContent,
    // TODO: Allow specifying constness for getters and setters separately?
    #[cfg(feature = "nightly")]
    uses_const_fns: bool,
}

impl Field {
    fn is_readable(&self) -> bool {
        match &self.content {
            FieldContent::Single(content) => !matches!(content.get_kind, AccessorKind::Disabled),
            FieldContent::Nested(content) => content.is_readable,
        }
    }

    fn has_const_getter(&self) -> bool {
        #[cfg(feature = "nightly")]
        {
            self.uses_const_fns
        }
        #[cfg(not(feature = "nightly"))]
        false
    }

    fn has_const_setter(&self) -> bool {
        #[cfg(feature = "nightly")]
        {
            self.uses_const_fns
        }
        #[cfg(not(feature = "nightly"))]
        false
    }

    fn has_unsafe_getter(&self) -> bool {
        match &self.content {
            FieldContent::Single(content) => content.get_kind.is_unsafe(),
            FieldContent::Nested(_) => false,
        }
    }

    fn asserts(
        &self,
        bits_span: &BitsSpan,
        full_bits: &proc_macro2::TokenStream,
        full_bits_are_const: bool,
    ) -> MaybeRepeat {
        let Field { ident, ty, .. } = self;

        let ty_bits = quote! { ::core::mem::size_of::<#ty>() << 3 };

        let assert = maybe_const_assert(full_bits_are_const);
        MaybeRepeat::new(
            match &bits_span {
                BitsSpan::Single(bit) => {
                    quote_spanned! {
                        ident.span() =>
                        #assert((#bit) < (#full_bits));
                    }
                }
                BitsSpan::Range { start, end } => {
                    quote_spanned! {
                        ident.span() =>
                        #assert((#end) > (#start));
                        #assert((#start) < (#full_bits) && (#end) <= (#full_bits));
                        #assert((#end) - (#start) <= (#ty_bits));
                    }
                }
                BitsSpan::Full => {
                    quote_spanned! {
                        ident.span() =>
                        #assert((#full_bits) <= (#ty_bits));
                    }
                }
            },
            full_bits_are_const,
        )
    }

    #[cfg(feature = "gce")]
    fn add_sized_preds(
        bits_span: &BitsSpan,
        full_bits: &proc_macro2::TokenStream,
        start_bit: &proc_macro2::TokenStream,
        end_bit: &proc_macro2::TokenStream,
        clause: &mut WhereClause,
    ) {
        match bits_span {
            BitsSpan::Single(bit) => {
                let bit = quote! { (#start_bit) + (#bit) };
                clause.predicates.push(sized_pred(&bit));
            }
            BitsSpan::Range { start, end } => {
                let start = quote! { (#start_bit) + (#start) };
                let end = min(&quote! { (#start_bit) + (#end) }, end_bit);
                clause.predicates.push(sized_pred(&start));
                clause.predicates.push(sized_pred(&end));
            }
            BitsSpan::Full => {
                let end = min(&quote! { (#start_bit) + (#full_bits) }, end_bit);
                clause.predicates.push(sized_pred(&end));
            }
        }
    }

    fn getters(
        &self,
        bits_span: &BitsSpan,
        asserts: &MaybeRepeat,
        storage: &proc_macro2::TokenStream,
        storage_ty: &Type,
        full_bits: &proc_macro2::TokenStream,
        #[cfg(feature = "gce")] start_end_bits: Option<(
            &proc_macro2::TokenStream,
            &proc_macro2::TokenStream,
        )>,
        #[cfg(feature = "gce")] storage_needs_const_bounds: bool,
    ) -> Option<proc_macro2::TokenStream> {
        let Field {
            attrs,
            vis,
            ident,
            ty: field_ty,
            content,
            ..
        } = self;

        #[allow(unused_mut)]
        let mut where_clause = WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        };
        #[cfg(feature = "gce")]
        if let Some((start_bit, end_bit)) = start_end_bits {
            Self::add_sized_preds(bits_span, full_bits, start_bit, end_bit, &mut where_clause);
        }

        match content {
            FieldContent::Single(SingleField { get_kind, .. }) => {
                if matches!(&get_kind, AccessorKind::Disabled) {
                    return None;
                }

                let (output, output_ty) = match get_kind {
                    AccessorKind::Default => (quote! { raw_value }, field_ty.to_token_stream()),

                    AccessorKind::ConvTy(ty) => (
                        quote! {
                            <#ty as ::core::convert::From<#field_ty>>::from(raw_value)
                        },
                        ty.to_token_stream(),
                    ),
                    AccessorKind::UnsafeConvTy {
                        ty,
                        has_safe_accessor,
                    } => {
                        let unsafe_ = has_safe_accessor.then(|| quote! { unsafe });
                        (
                            quote! {
                                #unsafe_ {
                                    <
                                        #ty as ::proc_bitfield::UnsafeFrom<#field_ty>
                                    >::unsafe_from(raw_value)
                                }
                            },
                            ty.to_token_stream(),
                        )
                    }
                    AccessorKind::TryConvTy(ty) => (
                        quote! {
                            <
                                #ty as ::core::convert::TryFrom<#field_ty>
                            >::try_from(raw_value)
                        },
                        quote! {
                            ::core::result::Result<
                                #ty,
                                <#ty as ::core::convert::TryFrom<#field_ty>>::Error,
                            >
                        },
                    ),
                    AccessorKind::UnwrapConvTy(ty) => (
                        quote! {
                            <
                                #ty as ::core::convert::TryFrom<#field_ty>
                            >::try_from(raw_value).unwrap()
                        },
                        ty.to_token_stream(),
                    ),

                    AccessorKind::ConvFn { fn_, ty } => {
                        (quote! { #fn_(raw_value) }, ty.to_token_stream())
                    }
                    AccessorKind::UnsafeConvFn {
                        fn_,
                        ty,
                        has_safe_accessor,
                    } => {
                        let unsafe_ = has_safe_accessor.then(|| quote! { unsafe });
                        (
                            quote! { #unsafe_ { #fn_(raw_value) } },
                            ty.to_token_stream(),
                        )
                    }
                    AccessorKind::TryGetFn { fn_, result_ty } => {
                        (quote! { #fn_(raw_value) }, result_ty.to_token_stream())
                    }
                    AccessorKind::UnwrapConvFn { fn_, ty } => {
                        (quote! { #fn_(raw_value).unwrap() }, ty.to_token_stream())
                    }

                    AccessorKind::TrySetFn { .. } | AccessorKind::Disabled => unreachable!(),
                };

                #[allow(unused_labels)]
                let get_raw_value = 'get_raw_value: {
                    #[cfg(feature = "gce")]
                    if let Some((start_bit, end_bit)) = start_end_bits {
                        break 'get_raw_value match bits_span {
                            BitsSpan::Single(bit) => {
                                let bit = quote! { (#start_bit) + (#bit) };
                                quote_spanned! {
                                    ident.span() =>
                                    if (#bit) < (#end_bit) {
                                        <#storage_ty as ::proc_bitfield::Bit>::bit::<{#bit}>(&#storage)
                                    } else {
                                        false
                                    }
                                }
                            }
                            BitsSpan::Range { start, end } => {
                                let start = quote! { (#start_bit) + (#start) };
                                let end = min(&quote! { (#start_bit) + (#end) }, end_bit);
                                quote_spanned! {
                                    ident.span() =>
                                    <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                        ::bits::<{#start}, {#end}>(&#storage)
                                }
                            }
                            BitsSpan::Full => {
                                let end = min(&quote! { (#start_bit) + (#full_bits) }, end_bit);
                                quote_spanned! {
                                    ident.span() =>
                                    <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                        ::bits::<{#start_bit}, {#end}>(&#storage)
                                }
                            }
                        };
                    }

                    match bits_span {
                        BitsSpan::Single(bit) => {
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::Bit>::bit::<{#bit}>(&#storage)
                            }
                        }
                        BitsSpan::Range { start, end } => {
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                    ::bits::<{#start}, {#end}>(&#storage)
                            }
                        }
                        BitsSpan::Full => {
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                    ::bits::<0, {#full_bits}>(&#storage)
                            }
                        }
                    }
                };

                #[cfg(feature = "gce")]
                if self.has_const_getter() && storage_needs_const_bounds {
                    add_const_bounds(
                        ident.span(),
                        &mut where_clause,
                        storage_ty,
                        &[if matches!(bits_span, BitsSpan::Single(_)) {
                            quote! { ::proc_bitfield::Bit }
                        } else {
                            quote! { ::proc_bitfield::Bits<#field_ty> }
                        }],
                    );
                }

                let unsafe_ = get_kind.is_unsafe().then(|| quote! { unsafe });
                let const_ = self.has_const_getter().then(|| quote! { const });
                let asserts = asserts.get();
                Some(quote! {
                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #[allow(unused_braces)]
                    #vis #const_ #unsafe_ fn #ident(&self) -> #output_ty #where_clause {
                        #asserts
                        let raw_value = #get_raw_value;
                        #output
                    }
                })
            }

            FieldContent::Nested(NestedField { is_readable, .. }) => {
                if !*is_readable {
                    return None;
                }

                let (start, end) = bits_span.to_start_end_or_full(full_bits);
                #[cfg(feature = "gce")]
                let (start, end) = if let Some((start_bit, end_bit)) = start_end_bits {
                    let start = quote! { (#start_bit) + (#start) };
                    let end = min(&quote! { (#start_bit) + (#end) }, end_bit);
                    (Cow::Owned(start), Cow::Owned(end))
                } else {
                    (start, end)
                };

                #[cfg(feature = "gce")]
                let ref_getter = {
                    let mut where_clause = where_clause.clone();
                    let where_const = self.has_const_getter().then(|| quote! { [const] });
                    where_clause.predicates.push(
                        syn::parse(
                            quote! {
                                #field_ty: #where_const ::proc_bitfield::NestableBitfield<
                                    #storage_ty, {#start}, {#end}
                                >
                            }
                            .into(),
                        )
                        .unwrap(),
                    );

                    let ref_fn_ident = format_ident!("{}_ref", ident);
                    let const_ = self.has_const_getter().then(|| quote! { const });
                    let asserts = asserts.get();
                    quote! {
                        #(#attrs)*
                        #[inline]
                        #[allow(clippy::identity_op)]
                        #[allow(unused_braces)]
                        #vis #const_ fn #ref_fn_ident(&'_ self)
                            -> <#field_ty as ::proc_bitfield::NestableBitfield<
                                #storage_ty, {#start}, {#end}
                            >>::Nested<'_> #where_clause {
                            #asserts
                            ::proc_bitfield::__private::NestedBitfield::__from_storage(&#storage)
                        }
                    }
                };
                #[cfg(not(feature = "gce"))]
                let ref_getter = quote! {};

                #[cfg(feature = "gce")]
                if self.has_const_getter() && storage_needs_const_bounds {
                    add_const_bounds(
                        ident.span(),
                        &mut where_clause,
                        storage_ty,
                        &[quote! {
                            ::proc_bitfield::Bits<
                                <#field_ty as ::proc_bitfield::Bitfield>::Storage
                            >
                        }],
                    );
                }

                let const_ = self.has_const_getter().then(|| quote! { const });
                let asserts = asserts.get();
                Some(quote! {
                    #ref_getter

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #[allow(unused_braces)]
                    #vis #const_ fn #ident(&self) -> #field_ty #where_clause {
                        #asserts
                        let raw_value = <#storage_ty as ::proc_bitfield::Bits<
                                <#field_ty as ::proc_bitfield::Bitfield>::Storage
                            >>::bits::<{#start}, {#end}>(&#storage);
                        <#field_ty>::__from_storage(raw_value)
                    }
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn setters(
        &self,
        bits_span: &BitsSpan,
        asserts: &MaybeRepeat,
        storage: &proc_macro2::TokenStream,
        storage_ty: &Type,
        full_bits: &proc_macro2::TokenStream,
        _outer_is_readable: bool,
        outer_allows_with: bool,
        #[cfg(feature = "gce")] start_end_bits: Option<(
            &proc_macro2::TokenStream,
            &proc_macro2::TokenStream,
        )>,
        #[cfg(feature = "gce")] storage_needs_const_bounds: bool,
    ) -> Option<proc_macro2::TokenStream> {
        let Field {
            attrs,
            vis,
            ident,
            ty: field_ty,
            content,
            ..
        } = self;

        #[allow(unused_mut)]
        let mut where_clause = WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        };
        #[cfg(feature = "gce")]
        if let Some((start_bit, end_bit)) = start_end_bits {
            Self::add_sized_preds(bits_span, full_bits, start_bit, end_bit, &mut where_clause);
        }

        match content {
            FieldContent::Single(SingleField { set_kind, .. }) => {
                if matches!(&set_kind, AccessorKind::Disabled) {
                    return None;
                }

                let set_fn_ident = format_ident!("set_{}", ident);
                let with_fn_ident = format_ident!("with_{}", ident);

                let (input_ty, raw_value, set_ok, set_output_ty, with_ok, with_output_ty) =
                    match set_kind {
                        AccessorKind::Default => (
                            field_ty,
                            quote! { value },
                            quote! {},
                            quote! { () },
                            quote! { result },
                            quote! { Self },
                        ),

                        AccessorKind::ConvTy(ty) => (
                            ty,
                            quote! { <#ty as ::core::convert::Into<#field_ty>>::into(value) },
                            quote! {},
                            quote! { () },
                            quote! { result },
                            quote! { Self },
                        ),
                        AccessorKind::UnsafeConvTy {
                            ty,
                            has_safe_accessor,
                        } => {
                            let unsafe_ = has_safe_accessor.then(|| quote! { unsafe });
                            (
                                ty,
                                quote! {
                                    #unsafe_ {
                                        <
                                            #ty as ::proc_bitfield::UnsafeInto<#field_ty>
                                        >::unsafe_into(value)
                                    }
                                },
                                quote! {},
                                quote! { () },
                                quote! { result },
                                quote! { Self },
                            )
                        }
                        AccessorKind::TryConvTy(ty) => (
                            ty,
                            quote! {
                                <#ty as ::core::convert::TryInto<#field_ty>>::try_into(value)?
                            },
                            quote! { ::core::result::Result::Ok(()) },
                            quote! {
                                ::core::result::Result<
                                    (),
                                    <#ty as ::core::convert::TryInto<#field_ty>>::Error
                                >
                            },
                            quote! { ::core::result::Result::Ok(result) },
                            quote! {
                                ::core::result::Result<
                                    Self,
                                    <#ty as ::core::convert::TryInto<#field_ty>>::Error
                                >
                            },
                        ),
                        AccessorKind::UnwrapConvTy(ty) => (
                            ty,
                            quote! {
                                <#ty as ::core::convert::TryInto<#field_ty>>::try_into(value)
                                    .unwrap()
                            },
                            quote! {},
                            quote! { () },
                            quote! { result },
                            quote! { Self },
                        ),

                        AccessorKind::ConvFn { fn_, ty } => (
                            ty,
                            quote! { #fn_(value) },
                            quote! {},
                            quote! { () },
                            quote! { result },
                            quote! { Self },
                        ),
                        AccessorKind::UnsafeConvFn {
                            fn_,
                            ty,
                            has_safe_accessor,
                        } => {
                            let unsafe_ = has_safe_accessor.then(|| quote! { unsafe });
                            (
                                ty,
                                quote! { #unsafe_ { #fn_(value) } },
                                quote! {},
                                quote! { () },
                                quote! { result },
                                quote! { Self },
                            )
                        }
                        AccessorKind::TrySetFn {
                            fn_,
                            input_ty,
                            result_ty,
                        } => (
                            input_ty,
                            quote! { #fn_(value)? },
                            quote! {
                                <
                                    #result_ty as ::proc_bitfield::Try
                                >::WithOutput::<()>::from_output(())
                            },
                            quote! { <#result_ty as ::proc_bitfield::Try>::WithOutput<()> },
                            quote! {
                                <
                                    #result_ty as ::proc_bitfield::Try
                                >::WithOutput::<Self>::from_output(result)
                            },
                            quote! {
                                <#result_ty as ::proc_bitfield::Try>::WithOutput<Self>
                            },
                        ),
                        AccessorKind::UnwrapConvFn { fn_, ty } => (
                            ty,
                            quote! { #fn_(value).unwrap() },
                            quote! {},
                            quote! { () },
                            quote! { result },
                            quote! { Self },
                        ),

                        AccessorKind::TryGetFn { .. } | AccessorKind::Disabled => unreachable!(),
                    };

                #[allow(unused_labels)]
                let (with_raw_value, set_raw_value) = 'with_set_raw_value: {
                    #[cfg(feature = "gce")]
                    if let Some((_start_bit, _end_bit)) = start_end_bits {
                        break 'with_set_raw_value match bits_span {
                            BitsSpan::Single(bit) => (
                                quote_spanned! {
                                    ident.span() =>
                                    if (#_start_bit) + (#bit) < (#_end_bit) {
                                        <#storage_ty as ::proc_bitfield::WithBit>
                                            ::with_bit::<{#bit}>(#storage, #raw_value)
                                    } else {
                                        #storage
                                    }
                                },
                                quote_spanned! {
                                    ident.span() =>
                                    if (#_start_bit) + (#bit) < (#_end_bit) {
                                        <#storage_ty as ::proc_bitfield::SetBit>
                                            ::set_bit::<{#bit}>(&mut #storage, #raw_value)
                                    }
                                },
                            ),
                            BitsSpan::Range { start, end } => {
                                let start = quote! { (#_start_bit) + (#start) };
                                let end = min(&quote! { (#_start_bit) + (#end) }, _end_bit);
                                (
                                    quote_spanned! {
                                        ident.span() =>
                                        <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                                            ::with_bits::<{#start}, {#end}>(#storage, #raw_value)
                                    },
                                    quote_spanned! {
                                        ident.span() =>
                                        <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                                            ::set_bits::<{#start}, {#end}>(&mut #storage, #raw_value)
                                    },
                                )
                            }
                            BitsSpan::Full => {
                                let end = min(&quote! { (#_start_bit) + (#full_bits) }, _end_bit);
                                (
                                    quote_spanned! {
                                        ident.span() =>
                                        <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                                            ::with_bits::<0, {#end}>(#storage, #raw_value)
                                    },
                                    quote_spanned! {
                                        ident.span() =>
                                        <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                                            ::set_bits::<{#_start_bit}, {#end}>(
                                                &mut #storage,
                                                #raw_value,
                                            )
                                    },
                                )
                            }
                        };
                    }

                    match bits_span {
                        BitsSpan::Single(bit) => (
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::WithBit>
                                    ::with_bit::<{#bit}>(#storage, #raw_value)
                            },
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::SetBit>
                                    ::set_bit::<{#bit}>(&mut #storage, #raw_value)
                            },
                        ),
                        BitsSpan::Range { start, end } => (
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                                    ::with_bits::<{#start}, {#end}>(#storage, #raw_value)
                            },
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                                    ::set_bits::<{#start}, {#end}>(&mut #storage, #raw_value)
                            },
                        ),
                        BitsSpan::Full => (
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                                    ::with_bits::<0, {#full_bits}>(#storage, #raw_value)
                            },
                            quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                                    ::set_bits::<0, {#full_bits}>(&mut #storage, #raw_value)
                            },
                        ),
                    }
                };

                let modifier = outer_allows_with.then(|| {
                    #[cfg(feature = "gce")]
                    if self.has_const_setter() && storage_needs_const_bounds {
                        add_const_bounds(
                            ident.span(),
                            &mut where_clause,
                            storage_ty,
                            &[if matches!(bits_span, BitsSpan::Single(_)) {
                                quote! { ::proc_bitfield::WithBit }
                            } else {
                                quote! { ::proc_bitfield::WithBits<#field_ty> }
                            }],
                        );
                    }

                    let unsafe_ = set_kind.is_unsafe().then(|| quote! { unsafe });
                    let const_ = self.has_const_setter().then(|| quote! { const });
                    let asserts = asserts.get();
                    quote! {
                        #(#attrs)*
                        #[inline]
                        #[must_use]
                        #[allow(clippy::identity_op)]
                        #[allow(unused_braces)]
                        #vis #const_ #unsafe_ fn #with_fn_ident(self, value: #input_ty)
                            -> #with_output_ty #where_clause
                        {
                            #asserts
                            let result = Self::__from_storage(#with_raw_value);
                            #with_ok
                        }
                    }
                });

                #[cfg(feature = "gce")]
                if self.has_const_setter() && storage_needs_const_bounds {
                    add_const_bounds(
                        ident.span(),
                        &mut where_clause,
                        storage_ty,
                        &[if matches!(bits_span, BitsSpan::Single(_)) {
                            quote! { ::proc_bitfield::SetBit }
                        } else {
                            quote! { ::proc_bitfield::SetBits<#field_ty> }
                        }],
                    );
                }

                let unsafe_ = set_kind.is_unsafe().then(|| quote! { unsafe });
                let const_ = self.has_const_setter().then(|| quote! { const });
                let asserts = asserts.get();
                Some(quote! {
                    #modifier

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #[allow(unused_braces)]
                    #vis #const_ #unsafe_ fn #set_fn_ident(&mut self, value: #input_ty)
                        -> #set_output_ty #where_clause
                    {
                        #asserts
                        #set_raw_value;
                        #set_ok
                    }
                })
            }

            FieldContent::Nested(NestedField {
                is_readable: _is_readable,
                is_writable,
            }) => {
                if !*is_writable {
                    return None;
                }

                let (start, end) = bits_span.to_start_end_or_full(full_bits);

                let set_fn_ident = format_ident!("set_{}", ident);
                let with_fn_ident = format_ident!("with_{}", ident);

                #[cfg(feature = "gce")]
                let mut_getter = (_outer_is_readable && *_is_readable).then(|| {
                    let mut where_clause = where_clause.clone();
                    let where_const = self.has_const_setter().then(|| quote! { [const] });
                    where_clause.predicates.push(
                        syn::parse(
                            quote! {
                                #field_ty: #where_const ::proc_bitfield::NestableMutBitfield<
                                    #storage_ty, {#start}, {#end}
                                >
                            }
                            .into(),
                        )
                        .unwrap(),
                    );

                    let mut_fn_ident = format_ident!("{}_mut", ident);
                    let const_ = self.has_const_setter().then(|| quote! { const });
                    let asserts = asserts.get();
                    quote! {
                        #(#attrs)*
                        #[inline]
                        #[allow(clippy::identity_op)]
                        #[allow(unused_braces)]
                        #vis #const_ fn #mut_fn_ident(&'_ mut self)
                            -> <#field_ty as ::proc_bitfield::NestableMutBitfield<
                                #storage_ty, {#start}, {#end}
                            >>::NestedMut<'_> #where_clause
                        {
                            #asserts
                            ::proc_bitfield::__private::NestedMutBitfield::__from_storage(
                                &mut #storage,
                            )
                        }
                    }
                });
                #[cfg(not(feature = "gce"))]
                let mut_getter = quote! {};

                #[cfg(feature = "gce")]
                let writer = {
                    let mut where_clause = where_clause.clone();
                    let where_const = self.has_const_setter().then(|| quote! { [const] });
                    where_clause.predicates.push(
                        syn::parse(
                            quote! {
                                #field_ty: #where_const ::proc_bitfield::NestableWriteBitfield<
                                    #storage_ty, {#start}, {#end}
                                >
                            }
                            .into(),
                        )
                        .unwrap(),
                    );

                    let write_fn_ident = format_ident!("{}_write", ident);
                    let const_ = self.has_const_setter().then(|| quote! { const });
                    let asserts = asserts.get();
                    quote! {
                        #(#attrs)*
                        #[inline]
                        #[allow(clippy::identity_op)]
                        #[allow(unused_braces)]
                        #vis #const_ fn #write_fn_ident(&'_ mut self)
                            -> <#field_ty as ::proc_bitfield::NestableWriteBitfield<
                                #storage_ty, {#start}, {#end}
                            >>::NestedWrite<'_> #where_clause
                        {
                            #asserts
                            ::proc_bitfield::__private::NestedWriteBitfield::__from_storage(
                                &mut #storage,
                            )
                        }
                    }
                };
                #[cfg(not(feature = "gce"))]
                let writer = quote! {};

                let modifier = outer_allows_with.then(|| {
                    #[cfg(feature = "gce")]
                    if self.has_const_setter() && storage_needs_const_bounds {
                        add_const_bounds(
                            ident.span(),
                            &mut where_clause,
                            storage_ty,
                            &[quote! {
                                ::proc_bitfield::WithBits<
                                    <#field_ty as ::proc_bitfield::Bitfield>::Storage
                                >
                            }],
                        );
                    }

                    let const_ = self.has_const_setter().then(|| quote! { const });
                    let asserts = asserts.get();
                    quote! {
                        #(#attrs)*
                        #[inline]
                        #[must_use]
                        #[allow(clippy::identity_op)]
                        #[allow(unused_braces)]
                        #vis #const_ fn #with_fn_ident(self, value: #field_ty)
                            -> Self #where_clause
                        {
                            #asserts
                            Self::__from_storage(
                                <#storage_ty as ::proc_bitfield::WithBits<
                                    <#field_ty as ::proc_bitfield::Bitfield>::Storage
                                >>::with_bits::<{#start}, {#end}>(#storage, value.0),
                            )
                        }
                    }
                });

                #[cfg(feature = "gce")]
                if self.has_const_setter() && storage_needs_const_bounds {
                    add_const_bounds(
                        ident.span(),
                        &mut where_clause,
                        storage_ty,
                        &[quote! {
                            ::proc_bitfield::SetBits<
                                <#field_ty as ::proc_bitfield::Bitfield>::Storage
                            >
                        }],
                    );
                }

                let const_ = self.has_const_setter().then(|| quote! { const });
                let asserts = asserts.get();
                Some(quote! {
                    #mut_getter
                    #writer

                    #modifier

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #[allow(unused_braces)]
                    #vis #const_ fn #set_fn_ident(&mut self, value: #field_ty) #where_clause {
                        #asserts
                        <#storage_ty as ::proc_bitfield::SetBits<
                            <#field_ty as ::proc_bitfield::Bitfield>::Storage>
                        >::set_bits::<{#start}, {#end}>(&mut #storage, value.0);
                    }
                })
            }
        }
    }
}

struct AutoImpls {
    debug: bool,
    from_storage: bool,
    into_storage: bool,
    deref_storage: bool,
}

struct Struct {
    outer_attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    storage_vis: Visibility,
    storage_ty: Type,
    auto_impls: AutoImpls,
    fields: Punctuated<Field, Token![,]>,
}

fn check_has_nightly(span: Span) -> syn::Result<()> {
    if !cfg!(feature = "nightly") {
        return Err(Error::new(
            span,
            "const fns can only be generated with the \"nightly\" feature enabled",
        ));
    }
    Ok(())
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;

        let mut generics = input.parse::<Generics>()?;

        let (storage_vis, storage_ty) = {
            let content = parse_parens(input)?;
            (content.parse()?, content.parse()?)
        };

        let mut auto_impls = AutoImpls {
            debug: false,
            from_storage: false,
            into_storage: false,
            deref_storage: false,
        };
        #[cfg(feature = "nightly")]
        let mut fields_use_const_fns_by_default = false;
        if input.parse::<Token![:]>().is_ok() {
            loop {
                if input.is_empty() {
                    break;
                }
                if let Ok(kw) = input.parse::<Token![const]>() {
                    check_has_nightly(kw.span())?;
                    #[cfg(feature = "nightly")]
                    {
                        fields_use_const_fns_by_default = true;
                    }
                } else if input.parse::<kw::Debug>().is_ok() {
                    auto_impls.debug = true;
                } else if input.parse::<kw::FromStorage>().is_ok() {
                    auto_impls.from_storage = true;
                } else if input.parse::<kw::IntoStorage>().is_ok() {
                    auto_impls.into_storage = true;
                } else if input.parse::<kw::DerefStorage>().is_ok() {
                    auto_impls.deref_storage = true;
                } else {
                    break;
                }
                if input.parse::<Token![,]>().is_err() {
                    break;
                }
            }
        }

        if input.peek(Token![where]) {
            *generics.make_where_clause() = input.parse()?;
        };

        let lookahead = input.lookahead1();
        if !lookahead.peek(token::Brace) {
            return Err(lookahead.error());
        }

        let content = parse_braces(input)?;
        assert!(
            content.call(Attribute::parse_inner)?.is_empty(),
            "Inner attributes are not supported right now"
        );
        let fields = parse_terminated(&content, |input: ParseStream| {
            let attrs = input.call(Attribute::parse_outer)?;
            let vis = input.parse()?;
            let ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let is_nested = input.parse::<kw::nested>().is_ok();
            let ty = input.parse::<Type>()?;

            #[cfg(feature = "nightly")]
            let mut uses_const_fns = fields_use_const_fns_by_default;

            let content = if is_nested {
                let mut is_readable = true;
                let mut is_writable = true;
                if let Ok(options_content) = parse_brackets(input) {
                    macro_rules! check_accessor_conflict {
                        ($ident: ident, $name: literal, $other: ident, $span: ident) => {
                            if !$ident {
                                return Err(Error::new(
                                    $span,
                                    concat!("Duplicate ", $name, " specifiers"),
                                ));
                            }
                            if !$other {
                                return Err(Error::new(
                                    $span,
                                    "Conflicting read_only and write_only specifiers",
                                ));
                            }
                        };
                    }

                    while !options_content.is_empty() {
                        let lookahead = options_content.lookahead1();
                        if lookahead.peek(Token![const]) {
                            check_has_nightly(input.span())?;
                            #[cfg(feature = "nightly")]
                            {
                                uses_const_fns = true;
                            }
                        } else if lookahead.peek(kw::no_const) {
                            #[cfg(feature = "nightly")]
                            {
                                uses_const_fns = false;
                            }
                        } else if lookahead.peek(kw::read_only) || lookahead.peek(kw::ro) {
                            let span = options_content
                                .parse::<kw::read_only>()
                                .map(|kw| kw.span)
                                .or_else(|_| {
                                options_content.parse::<kw::ro>().map(|kw| kw.span)
                            })?;
                            check_accessor_conflict!(is_writable, "read_only", is_readable, span);
                            is_writable = false;
                        } else if lookahead.peek(kw::write_only) || lookahead.peek(kw::wo) {
                            let span = options_content
                                .parse::<kw::write_only>()
                                .map(|kw| kw.span)
                                .or_else(|_| options_content.parse::<kw::wo>().map(|kw| kw.span))?;
                            check_accessor_conflict!(is_readable, "write_only", is_writable, span);
                            is_readable = false;
                        } else {
                            return Err(lookahead.error());
                        }

                        let had_comma = options_content.parse::<Token![,]>().is_ok();
                        if !options_content.is_empty() && !had_comma {
                            return Err(
                                options_content.error("expected comma between field options")
                            );
                        }
                    }
                }
                FieldContent::Nested(NestedField {
                    is_readable,
                    is_writable,
                })
            } else {
                let mut get = AccessorKind::Default;
                let mut set = AccessorKind::Default;
                if let Ok(options_content) = parse_brackets(input) {
                    macro_rules! check_conversion_ty_conflict {
                                ($($ident: ident),*; $span: expr) => {
                                    if $(!matches!(&$ident, AccessorKind::Default))||* {
                                        return Err(Error::new(
                                            $span,
                                            "Conflicting conversion type definitions",
                                        ));
                                    }
                                };
                            }

                    macro_rules! check_accessor_conflict {
                        ($ident: ident, $name: literal, $other: ident, $span: ident) => {
                            if matches!(&$ident, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    $span,
                                    concat!("Duplicate ", $name, " specifiers"),
                                ));
                            }
                            if matches!(&$other, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    $span,
                                    "Conflicting read_only and write_only specifiers",
                                ));
                            }
                        };
                    }

                    fn parse_return_ty(input: ParseStream) -> Result<Result<Type>> {
                        if let Err(err) = input.parse::<Token![->]>() {
                            return Ok(Err(err));
                        }
                        Ok(input.parse())
                    }

                    fn parse_parenthesized_ty(input: ParseStream) -> Result<Result<Type>> {
                        Ok(match parse_parens(input) {
                            Ok(content) => content.parse(),
                            Err(err) => Err(err),
                        })
                    }

                    while !options_content.is_empty() {
                        if let Ok(kw) = options_content.parse::<Token![const]>() {
                            check_has_nightly(kw.span())?;
                            #[cfg(feature = "nightly")]
                            {
                                uses_const_fns = true;
                            }
                        } else if options_content.parse::<kw::no_const>().is_ok() {
                            #[cfg(feature = "nightly")]
                            {
                                uses_const_fns = false;
                            }
                        }
                        // Infallible conversions
                        else if let Ok(kw) = options_content.parse::<kw::get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            get = AccessorKind::ConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            set = AccessorKind::ConvTy(options_content.parse()?);
                        }
                        // Unsafe conversions
                        else if let Ok(kw) = options_content.parse::<kw::unsafe_get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            get = AccessorKind::UnsafeConvTy {
                                ty: options_content.parse()?,
                                has_safe_accessor,
                            };
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            set = AccessorKind::UnsafeConvTy {
                                ty: options_content.parse()?,
                                has_safe_accessor,
                            };
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_both>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnsafeConvTy {
                                ty: ty.clone(),
                                has_safe_accessor,
                            };
                            set = AccessorKind::UnsafeConvTy {
                                ty,
                                has_safe_accessor,
                            };
                        } else if let Ok(kw) = options_content.parse::<Token![unsafe]>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnsafeConvTy {
                                ty: ty.clone(),
                                has_safe_accessor,
                            };
                            set = AccessorKind::ConvTy(ty);
                        }
                        // Fallible conversions
                        else if let Ok(kw) = options_content.parse::<kw::try_get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            get = AccessorKind::TryConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::try_set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            set = AccessorKind::TryConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::try_both>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::TryConvTy(ty.clone());
                            set = AccessorKind::TryConvTy(ty);
                        } else if let Ok(kw) = options_content.parse::<Token![try]>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::TryConvTy(ty.clone());
                            set = AccessorKind::ConvTy(ty);
                        }
                        // Unwrapping conversions
                        else if let Ok(kw) = options_content.parse::<kw::unwrap_get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            get = AccessorKind::UnwrapConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::unwrap_set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            set = AccessorKind::UnwrapConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::unwrap_both>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnwrapConvTy(ty.clone());
                            set = AccessorKind::UnwrapConvTy(ty);
                        } else if let Ok(kw) = options_content.parse::<kw::unwrap>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnwrapConvTy(ty.clone());
                            set = AccessorKind::ConvTy(ty);
                        }
                        // Infallible fn conversions
                        else if let Ok(kw) = options_content.parse::<kw::get_fn>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty =
                                parse_return_ty(&options_content)?.unwrap_or_else(|_| ty.clone());
                            get = AccessorKind::ConvFn { fn_, ty };
                        } else if let Ok(kw) = options_content.parse::<kw::set_fn>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty = parse_parenthesized_ty(&options_content)?
                                .unwrap_or_else(|_| ty.clone());
                            set = AccessorKind::ConvFn { fn_, ty };
                        }
                        // Unsafe fn conversions
                        else if let Ok(kw) = options_content.parse::<kw::unsafe_get_fn>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty =
                                parse_return_ty(&options_content)?.unwrap_or_else(|_| ty.clone());
                            get = AccessorKind::UnsafeConvFn {
                                fn_,
                                ty,
                                has_safe_accessor,
                            };
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_set_fn>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let has_safe_accessor = options_content.parse::<Token![!]>().is_ok();
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty = parse_parenthesized_ty(&options_content)?
                                .unwrap_or_else(|_| ty.clone());
                            set = AccessorKind::UnsafeConvFn {
                                fn_,
                                ty,
                                has_safe_accessor,
                            };
                        }
                        // Fallible fn conversions
                        else if let Ok(kw) = options_content.parse::<kw::try_get_fn>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let result_ty = parse_return_ty(&options_content)??;
                            get = AccessorKind::TryGetFn { fn_, result_ty };
                        } else if let Ok(kw) = options_content.parse::<kw::try_set_fn>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let input_ty = parse_parenthesized_ty(&options_content)?
                                .unwrap_or_else(|_| ty.clone());
                            let result_ty = parse_return_ty(&options_content)??;
                            set = AccessorKind::TrySetFn {
                                fn_,
                                input_ty,
                                result_ty,
                            };
                        }
                        // Unwrapping fn conversions
                        else if let Ok(kw) = options_content.parse::<kw::unwrap_get_fn>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty =
                                parse_return_ty(&options_content)?.unwrap_or_else(|_| ty.clone());
                            get = AccessorKind::UnwrapConvFn { fn_, ty };
                        } else if let Ok(kw) = options_content.parse::<kw::unwrap_set_fn>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty = parse_parenthesized_ty(&options_content)?
                                .unwrap_or_else(|_| ty.clone());
                            set = AccessorKind::UnwrapConvFn { fn_, ty };
                        }
                        // Access restrictions
                        else if let Ok(span) = options_content
                            .parse::<kw::read_only>()
                            .map(|kw| kw.span)
                            .or_else(|_| options_content.parse::<kw::ro>().map(|kw| kw.span))
                        {
                            check_accessor_conflict!(set, "read_only", get, span);
                            set = AccessorKind::Disabled;
                        } else if let Ok(span) = options_content
                            .parse::<kw::write_only>()
                            .map(|kw| kw.span)
                            .or_else(|_| options_content.parse::<kw::wo>().map(|kw| kw.span))
                        {
                            check_accessor_conflict!(get, "write_only", set, span);
                            get = AccessorKind::Disabled;
                        }
                        // Infallible conversion (without keywords)
                        else {
                            let ty: Type = options_content.parse()?;
                            check_conversion_ty_conflict!(get, set; ty.span());
                            get = AccessorKind::ConvTy(ty.clone());
                            set = AccessorKind::ConvTy(ty);
                        }

                        let had_comma = options_content.parse::<Token![,]>().is_ok();
                        if !options_content.is_empty() && !had_comma {
                            return Err(
                                options_content.error("expected comma between field options")
                            );
                        }
                    }
                }
                FieldContent::Single(SingleField {
                    get_kind: get,
                    set_kind: set,
                })
            };
            input.parse::<Token![@]>()?;
            let bits = input.parse()?;
            Ok(Field {
                attrs,
                vis,
                ident,
                bits,
                ty,
                content,
                #[cfg(feature = "nightly")]
                uses_const_fns,
            })
        })?;

        Ok(Struct {
            outer_attrs,
            vis,
            ident,
            generics,
            storage_vis,
            storage_ty,
            auto_impls,
            fields,
        })
    }
}

#[cfg(feature = "gce")]
fn add_nested_generics(mut impl_generics: Generics) -> Generics {
    impl_generics.params.insert(
        0,
        LifetimeParam {
            attrs: Vec::new(),
            lifetime: Lifetime::new("'_storage", Span::call_site()),
            colon_token: Default::default(),
            bounds: Default::default(),
        }
        .into(),
    );
    impl_generics
}

#[cfg(feature = "gce")]
fn add_nested_impl_generics(
    mut generics: Generics,
    fields: &Punctuated<Field, Token![,]>,
    outer_is_readable: bool,
    outer_is_writable: bool,
) -> Generics {
    let mut bounds = vec![TraitBound {
        paren_token: Default::default(),
        modifier: TraitBoundModifier::None,
        lifetimes: Default::default(),
        path: Path {
            leading_colon: Some(Default::default()),
            segments: [
                Ident::new("proc_bitfield", Span::call_site()).into(),
                PathSegment {
                    ident: Ident::new("Bit", Span::call_site()),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    }
    .into()];

    for field in fields {
        let Field {
            ty, content, bits, ..
        } = field;

        match content {
            FieldContent::Single(SingleField { get_kind, set_kind }) => {
                if matches!(bits, Bits::Single(_) | Bits::SinglePack { .. }) {
                    if !matches!(get_kind, AccessorKind::Disabled) && outer_is_readable {
                        bounds.push(
                            syn::parse::<TraitBound>(quote! { ::proc_bitfield::Bit }.into())
                                .unwrap()
                                .into(),
                        );
                    }

                    if !matches!(set_kind, AccessorKind::Disabled) && outer_is_writable {
                        bounds.push(
                            syn::parse::<TraitBound>(quote! { ::proc_bitfield::WithBit }.into())
                                .unwrap()
                                .into(),
                        );
                        bounds.push(
                            syn::parse::<TraitBound>(quote! { ::proc_bitfield::SetBit }.into())
                                .unwrap()
                                .into(),
                        );
                    }
                } else {
                    if !matches!(get_kind, AccessorKind::Disabled) && outer_is_readable {
                        bounds.push(
                            syn::parse::<TraitBound>(quote! { ::proc_bitfield::Bits<#ty> }.into())
                                .unwrap()
                                .into(),
                        );
                    }

                    if !matches!(set_kind, AccessorKind::Disabled) && outer_is_writable {
                        bounds.push(
                            syn::parse::<TraitBound>(
                                quote! { ::proc_bitfield::WithBits<#ty> }.into(),
                            )
                            .unwrap()
                            .into(),
                        );
                        bounds.push(
                            syn::parse::<TraitBound>(
                                quote! { ::proc_bitfield::SetBits<#ty> }.into(),
                            )
                            .unwrap()
                            .into(),
                        );
                    }
                }
            }
            FieldContent::Nested(NestedField {
                is_readable,
                is_writable,
            }) => {
                let field_storage_ty = quote! { <#ty as ::proc_bitfield::Bitfield>::Storage };
                if *is_readable && outer_is_readable {
                    bounds.push(
                        syn::parse::<TraitBound>(
                            quote! { ::proc_bitfield::Bits<#field_storage_ty> }.into(),
                        )
                        .unwrap()
                        .into(),
                    );
                }

                if *is_writable && outer_is_writable {
                    bounds.extend_from_slice(&[
                        syn::parse::<TraitBound>(
                            quote! { ::proc_bitfield::WithBits<#field_storage_ty> }.into(),
                        )
                        .unwrap()
                        .into(),
                        syn::parse::<TraitBound>(
                            quote! { ::proc_bitfield::SetBits<#field_storage_ty> }.into(),
                        )
                        .unwrap()
                        .into(),
                    ]);
                }
            }
        }
    }

    generics
        .params
        .push(type_param(Ident::new("_Storage", Span::call_site()), bounds, None).into());
    generics.params.push(
        const_param(
            Ident::new("START", Span::call_site()),
            TypePath {
                qself: None,
                path: Ident::new("usize", Span::call_site()).into(),
            }
            .into(),
            None,
        )
        .into(),
    );
    generics.params.push(
        const_param(
            Ident::new("END", Span::call_site()),
            TypePath {
                qself: None,
                path: Ident::new("usize", Span::call_site()).into(),
            }
            .into(),
            None,
        )
        .into(),
    );
    generics
}

#[allow(clippy::too_many_arguments)]
fn impl_bitfield_ty<'a>(
    outer_attrs: &[Attribute],
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    unused_type_params: impl IntoIterator<Item = &'a TypeParam>,
    storage_vis: &Visibility,
    storage_ty: &Type,
    storage_deref: &proc_macro2::TokenStream,
    storage_deref_ty: &Type,
    full_bits: &proc_macro2::TokenStream,
    full_bits_are_const: bool,
    fields: &Punctuated<Field, Token![,]>,
    is_readable: bool,
    is_writable: bool,
    allows_with: bool,
    #[cfg(feature = "gce")] start_end_bits: Option<(
        &proc_macro2::TokenStream,
        &proc_macro2::TokenStream,
    )>,
    #[cfg(feature = "gce")] storage_needs_const_bounds: bool,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut unused_type_params = unused_type_params.into_iter().peekable();
    let has_unused_type_params = unused_type_params.peek().is_some();
    let type_params_phantom_data = if has_unused_type_params {
        quote! { , ::core::marker::PhantomData }
    } else {
        quote! {}
    };

    let mut last_bits_span = None;
    let field_fns = fields
        .iter()
        .map(|field| {
            let bits_span = match field.bits.clone().into_span(last_bits_span.as_ref()) {
                Ok(bits_span) => bits_span,
                Err(err) => return err.to_compile_error(),
            };
            last_bits_span = Some(bits_span.clone());

            let asserts = field.asserts(&bits_span, full_bits, full_bits_are_const);

            let getters = is_readable.then(|| {
                field.getters(
                    &bits_span,
                    &asserts,
                    storage_deref,
                    storage_deref_ty,
                    full_bits,
                    #[cfg(feature = "gce")]
                    start_end_bits,
                    #[cfg(feature = "gce")]
                    storage_needs_const_bounds,
                )
            });

            let setters = is_writable.then(|| {
                field.setters(
                    &bits_span,
                    &asserts,
                    storage_deref,
                    storage_deref_ty,
                    full_bits,
                    is_readable,
                    allows_with,
                    #[cfg(feature = "gce")]
                    start_end_bits,
                    #[cfg(feature = "gce")]
                    storage_needs_const_bounds,
                )
            });

            quote! {
                #getters
                #setters
            }
        })
        .collect::<Vec<_>>();

    let type_params_phantom_data_field_ty = if has_unused_type_params {
        quote! { , ::core::marker::PhantomData<(#(#unused_type_params),*)> }
    } else {
        quote! {}
    };

    quote! {
        #(#outer_attrs)*
        #[repr(transparent)]
        #vis struct #ident #generics(
            #storage_vis #storage_ty #type_params_phantom_data_field_ty
        ) #where_clause;

        impl #impl_generics #ident #ty_generics #where_clause {
            #[inline(always)]
            const fn __from_storage(storage: #storage_ty) -> Self {
                Self(storage #type_params_phantom_data)
            }

            #(#field_fns)*
        }
    }
}

pub fn bitfield(input: TokenStream) -> TokenStream {
    let Struct {
        outer_attrs,
        vis,
        ident,
        generics,
        storage_vis,
        storage_ty,
        auto_impls,
        fields,
    } = syn::parse_macro_input!(input);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let ty = quote! { #ident #ty_generics };

    let storage_ty_bits = quote! { ::core::mem::size_of::<#storage_ty>() << 3 };
    let storage_ty_bits_are_const = generics.params.is_empty();

    let ty_impl = impl_bitfield_ty(
        &outer_attrs,
        &vis,
        &ident,
        &generics,
        generics.type_params(),
        &storage_vis,
        &storage_ty,
        &quote! { self.0 },
        &storage_ty,
        &storage_ty_bits,
        storage_ty_bits_are_const,
        &fields,
        true,
        true,
        true,
        #[cfg(feature = "gce")]
        None,
        #[cfg(feature = "gce")]
        false,
    );

    #[cfg(feature = "gce")]
    let nested = {
        let mut nested_outer_attrs = outer_attrs.clone();
        // TODO: Hack
        nested_outer_attrs.retain_mut(|attr| !attr.path().is_ident("derive"));
        nested_outer_attrs.push(doc_hidden());
        let nested_storage_deref_ty = Type::Path(TypePath {
            qself: None,
            path: Ident::new("_Storage", Span::call_site()).into(),
        });
        let nested_storage_lifetime = Lifetime::new("'_storage", Span::call_site());
        let nested_storage_deref = quote! { *self.0 };
        let nested_start_bit = quote! { START };
        let nested_end_bit = quote! { END };

        let nested_impl_generics_ =
            add_nested_impl_generics(generics.clone(), &fields, true, false);
        let (nested_impl_impl_generics, _, nested_impl_where_clause) =
            nested_impl_generics_.split_for_impl();

        let nested_ty_ident = format_ident!("__Nested_{ident}");
        let nested_generics = add_nested_generics(nested_impl_generics_.clone());
        let (nested_impl_generics, nested_ty_generics, nested_where_clause) =
            nested_generics.split_for_impl();
        let nested_ty_impl = impl_bitfield_ty(
            &nested_outer_attrs,
            &Visibility::Public(Default::default()),
            &nested_ty_ident,
            &nested_generics,
            generics.type_params(),
            &Visibility::Inherited,
            &TypeReference {
                and_token: Default::default(),
                lifetime: Some(nested_storage_lifetime.clone()),
                mutability: None,
                elem: Box::new(nested_storage_deref_ty.clone()),
            }
            .into(),
            &nested_storage_deref,
            &nested_storage_deref_ty,
            &storage_ty_bits,
            storage_ty_bits_are_const,
            &fields,
            true,
            false,
            false,
            Some((&nested_start_bit, &nested_end_bit)),
            true,
        );

        let nested_mut_impl_generics_ =
            add_nested_impl_generics(generics.clone(), &fields, true, true);
        let (nested_mut_impl_impl_generics, _, nested_mut_impl_where_clause) =
            nested_mut_impl_generics_.split_for_impl();

        let nested_mut_ty_ident = format_ident!("__Nested_Mut_{ident}");
        let nested_mut_generics = add_nested_generics(nested_mut_impl_generics_.clone());
        let (nested_mut_impl_generics, nested_mut_ty_generics, nested_mut_where_clause) =
            nested_mut_generics.split_for_impl();
        let nested_mut_ty_impl = impl_bitfield_ty(
            &nested_outer_attrs,
            &Visibility::Public(Default::default()),
            &nested_mut_ty_ident,
            &nested_mut_generics,
            generics.type_params(),
            &Visibility::Inherited,
            &TypeReference {
                and_token: Default::default(),
                lifetime: Some(nested_storage_lifetime.clone()),
                mutability: Some(Default::default()),
                elem: Box::new(nested_storage_deref_ty.clone()),
            }
            .into(),
            &nested_storage_deref,
            &nested_storage_deref_ty,
            &storage_ty_bits,
            storage_ty_bits_are_const,
            &fields,
            true,
            true,
            false,
            Some((&nested_start_bit, &nested_end_bit)),
            true,
        );

        let nested_write_impl_generics_ =
            add_nested_impl_generics(generics.clone(), &fields, false, true);
        let (nested_write_impl_impl_generics, _, nested_write_impl_where_clause) =
            nested_write_impl_generics_.split_for_impl();

        let nested_write_ty_ident = format_ident!("__Nested_Write_{ident}");
        let nested_write_generics = add_nested_generics(nested_write_impl_generics_.clone());
        let (nested_write_impl_generics, nested_write_ty_generics, nested_write_where_clause) =
            nested_write_generics.split_for_impl();
        let nested_write_ty_impl = impl_bitfield_ty(
            &nested_outer_attrs,
            &Visibility::Public(Default::default()),
            &nested_write_ty_ident,
            &nested_write_generics,
            generics.type_params(),
            &Visibility::Inherited,
            &TypeReference {
                and_token: Default::default(),
                lifetime: Some(nested_storage_lifetime.clone()),
                mutability: Some(Default::default()),
                elem: Box::new(nested_storage_deref_ty.clone()),
            }
            .into(),
            &nested_storage_deref,
            &nested_storage_deref_ty,
            &storage_ty_bits,
            storage_ty_bits_are_const,
            &fields,
            false,
            true,
            false,
            Some((&nested_start_bit, &nested_end_bit)),
            true,
        );

        quote! {
            impl #nested_impl_impl_generics ::proc_bitfield::NestableBitfield<
                _Storage, START, END
            >
                for #ty #nested_impl_where_clause
            {
                type Nested<'_storage> =
                    #nested_ty_ident #nested_ty_generics where _Storage: '_storage;
            }

            impl #nested_impl_generics const ::proc_bitfield::__private::NestedBitfield<
                '_storage, _Storage
            >
                for #nested_ty_ident #nested_ty_generics #nested_where_clause
            {
                #[inline(always)]
                fn __from_storage(storage: &'_storage _Storage) -> Self {
                    Self::__from_storage(storage)
                }
            }

            impl #nested_mut_impl_impl_generics ::proc_bitfield::NestableMutBitfield<
                _Storage, START, END
            >
                for #ty #nested_mut_impl_where_clause
            {
                type NestedMut<'_storage> =
                    #nested_mut_ty_ident #nested_mut_ty_generics where _Storage: '_storage;
            }

            impl #nested_mut_impl_generics const ::proc_bitfield::__private::NestedMutBitfield<
                '_storage, _Storage
            >
                for #nested_mut_ty_ident #nested_mut_ty_generics #nested_mut_where_clause
            {
                #[inline(always)]
                fn __from_storage(storage: &'_storage mut _Storage) -> Self {
                    Self::__from_storage(storage)
                }
            }

            impl #nested_write_impl_impl_generics ::proc_bitfield::NestableWriteBitfield<
                _Storage, START, END
            >
                for #ty #nested_write_impl_where_clause
            {
                type NestedWrite<'_storage> =
                    #nested_write_ty_ident #nested_write_ty_generics where _Storage: '_storage;
            }

            impl #nested_write_impl_generics const ::proc_bitfield::__private::NestedWriteBitfield<
                '_storage, _Storage
            >
                for #nested_write_ty_ident #nested_write_ty_generics #nested_write_where_clause
            {
                #[inline(always)]
                fn __from_storage(storage: &'_storage mut _Storage) -> Self {
                    Self::__from_storage(storage)
                }
            }

            #nested_ty_impl
            #nested_mut_ty_impl
            #nested_write_ty_impl
        }
    };
    #[cfg(not(feature = "gce"))]
    let nested = quote! {};

    let mut impls = Vec::new();

    if auto_impls.debug {
        let readable_fields = fields.iter().filter(|field| field.is_readable());
        let field_idents = readable_fields.clone().map(|field| &field.ident);
        let field_unsafes = readable_fields.map(|field| {
            if field.has_unsafe_getter() {
                quote! { unsafe }
            } else {
                quote! {}
            }
        });
        impls.push(quote! {
            impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    f.debug_struct(::core::stringify!(#ident))
                        .field("0", &self.0)
                        #(.field(
                            ::core::stringify!(#field_idents),
                            #field_unsafes { &self.#field_idents() },
                        ))*
                        .finish()
                }
            }
        });
    }

    if auto_impls.from_storage {
        impls.push(quote! {
            impl #impl_generics ::core::convert::From<#storage_ty> for #ident #ty_generics
                #where_clause
            {
                fn from(other: #storage_ty) -> Self {
                    Self::__from_storage(other)
                }
            }
        });
    }

    if auto_impls.into_storage {
        impls.push(quote! {
            impl #impl_generics ::core::convert::From<#ident #ty_generics> for #storage_ty
                #where_clause
            {
                fn from(other: #ident #ty_generics) -> Self {
                    other.0
                }
            }
        });
    }

    if auto_impls.deref_storage {
        impls.push(quote! {
            impl #impl_generics ::core::ops::Deref for #ident #ty_generics #where_clause {
                type Target = #storage_ty;

                fn deref(&self) -> &#storage_ty {
                    &self.0
                }
            }
        });
    }

    quote! {
        impl #impl_generics ::proc_bitfield::Bitfield for #ty #where_clause {
            type Storage = #storage_ty;
        }

        #ty_impl

        #nested

        #(#impls)*
    }
    .into()
}
