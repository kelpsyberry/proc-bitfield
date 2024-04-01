use super::{
    bits::{Bits, BitsSpan},
    utils::parse_parens,
};
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Error, Expr, ExprParen, ExprPath, Generics, Ident, Token, Type, Visibility,
    WhereClause,
};

mod kw {
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

    syn::custom_keyword!(Debug);
    syn::custom_keyword!(FromRaw);
    syn::custom_keyword!(IntoRaw);
    syn::custom_keyword!(DerefRaw);
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
    UnsafeConvTy(Type),
    UnsafeConvFn {
        fn_: Expr,
        ty: Type,
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
            AccessorKind::UnsafeConvTy(_) | AccessorKind::UnsafeConvFn { .. }
        )
    }
}

struct Field {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    ty: Type,
    get_kind: AccessorKind,
    set_kind: AccessorKind,
    bits: Bits,
}

struct AutoImpls {
    debug: bool,
    from_raw: bool,
    into_raw: bool,
    deref_raw: bool,
}

struct Struct {
    outer_attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    storage_vis: Visibility,
    storage_ty: Type,
    auto_impls: AutoImpls,
    generics: Generics,
    where_clause: Option<WhereClause>,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        let (storage_vis, storage_ty) = {
            let content;
            parenthesized!(content in input);
            (content.parse()?, content.parse()?)
        };

        let mut auto_impls = AutoImpls {
            debug: false,
            from_raw: false,
            into_raw: false,
            deref_raw: false,
        };
        if input.parse::<Token![:]>().is_ok() {
            loop {
                if input.is_empty() {
                    break;
                }
                if input.parse::<kw::Debug>().is_ok() {
                    auto_impls.debug = true;
                } else if input.parse::<kw::FromRaw>().is_ok() {
                    auto_impls.from_raw = true;
                } else if input.parse::<kw::IntoRaw>().is_ok() {
                    auto_impls.into_raw = true;
                } else if input.parse::<kw::DerefRaw>().is_ok() {
                    auto_impls.deref_raw = true;
                } else {
                    break;
                }
                if input.parse::<Token![,]>().is_err() {
                    break;
                }
            }
        }

        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Token![where]) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        };

        if where_clause.is_none() && lookahead.peek(token::Paren) {
            return Err(Error::new(input.span(), "Tuple structs are not supported"));
        } else if lookahead.peek(Token![;]) {
            return Err(Error::new(input.span(), "Empty structs are not supported"));
        } else if !lookahead.peek(token::Brace) {
            return Err(Error::new(input.span(), "Unknown struct type"));
        }

        let content;
        braced!(content in input);
        assert!(
            content.call(Attribute::parse_inner)?.is_empty(),
            "Inner attributes are not supported right now"
        );
        let fields = content.parse_terminated(
            |input| {
                let attrs = input.call(Attribute::parse_outer)?;
                let vis = input.parse()?;
                let ident = input.parse()?;
                input.parse::<Token![:]>()?;
                let ty = input.parse::<Type>()?;
                let mut get = AccessorKind::Default;
                let mut set = AccessorKind::Default;
                let lookahead = input.lookahead1();
                if lookahead.peek(token::Bracket) {
                    let options_content;
                    bracketed!(options_content in input);
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
                        // Infallible conversions
                        if let Ok(kw) = options_content.parse::<kw::get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            get = AccessorKind::ConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            set = AccessorKind::ConvTy(options_content.parse()?);
                        }
                        // Unsafe conversions
                        else if let Ok(kw) = options_content.parse::<kw::unsafe_get>() {
                            check_conversion_ty_conflict!(get; kw.span);
                            get = AccessorKind::UnsafeConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_set>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            set = AccessorKind::UnsafeConvTy(options_content.parse()?);
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_both>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnsafeConvTy(ty.clone());
                            set = AccessorKind::UnsafeConvTy(ty);
                        } else if let Ok(kw) = options_content.parse::<Token![unsafe]>() {
                            check_conversion_ty_conflict!(get, set; kw.span);
                            let ty: Type = options_content.parse()?;
                            get = AccessorKind::UnsafeConvTy(ty.clone());
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
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty =
                                parse_return_ty(&options_content)?.unwrap_or_else(|_| ty.clone());
                            get = AccessorKind::UnsafeConvFn { fn_, ty };
                        } else if let Ok(kw) = options_content.parse::<kw::unsafe_set_fn>() {
                            check_conversion_ty_conflict!(set; kw.span);
                            let fn_ = parse_accessor_fn(&options_content)?;
                            let ty = parse_parenthesized_ty(&options_content)?
                                .unwrap_or_else(|_| ty.clone());
                            set = AccessorKind::UnsafeConvFn { fn_, ty };
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
                            options_content.error("expected comma between field options");
                        }
                    }
                }
                input.parse::<Token![@]>()?;
                Ok(Field {
                    attrs,
                    vis,
                    ident,
                    ty,
                    get_kind: get,
                    set_kind: set,
                    bits: input.parse()?,
                })
            },
            Token![,],
        )?;

        Ok(Struct {
            outer_attrs,
            vis,
            ident,
            storage_vis,
            storage_ty,
            generics,
            auto_impls,
            where_clause,
            fields,
        })
    }
}

pub fn bitfield(input: TokenStream) -> TokenStream {
    let Struct {
        outer_attrs,
        vis,
        ident,
        storage_vis,
        storage_ty,
        auto_impls,
        generics,
        where_clause,
        fields,
    } = syn::parse_macro_input!(input);
    let field_fns = fields.iter().map(
        |Field {
             attrs,
             vis,
             ident,
             ty: field_ty,
             get_kind,
             set_kind,
             bits,
         }| {
            let storage_ty_bits = quote! { {::core::mem::size_of::<#storage_ty>() << 3} };
            let field_ty_bits = quote! { {::core::mem::size_of::<#field_ty>() << 3} };
            let set_fn_ident = format_ident!("set_{}", ident);
            let with_fn_ident = format_ident!("with_{}", ident);
            let bits_span = bits.clone().into_span();

            let getter = if !matches!(&get_kind, AccessorKind::Disabled) {
                let (calc_get_result, get_output_ty) = match get_kind {
                    AccessorKind::Default => (quote! { raw_result }, quote! { #field_ty }),
                    AccessorKind::Disabled => unreachable!(),

                    AccessorKind::ConvTy(get_ty) => (
                        quote! { <#get_ty as ::core::convert::From<#field_ty>>::from(raw_result) },
                        quote! { #get_ty },
                    ),
                    AccessorKind::UnsafeConvTy(get_ty) => (
                        quote! {
                            <#get_ty as ::proc_bitfield::UnsafeFrom<#field_ty>>::unsafe_from(
                                raw_result,
                            )
                        },
                        quote! { #get_ty },
                    ),
                    AccessorKind::TryConvTy(get_ty) => (
                        quote! {
                            <#get_ty as ::core::convert::TryFrom<#field_ty>>::try_from(raw_result)
                        },
                        quote! {
                            ::core::result::Result<
                                #get_ty,
                                <#get_ty as ::core::convert::TryFrom<#field_ty>>::Error,
                            >
                        },
                    ),
                    AccessorKind::UnwrapConvTy(get_ty) => (
                        quote! {
                            <#get_ty as ::core::convert::TryFrom<#field_ty>>::try_from(raw_result)
                                .unwrap()
                        },
                        quote! { #get_ty },
                    ),

                    AccessorKind::ConvFn { fn_, ty } => (
                        quote! { #fn_(raw_result) },
                        quote! { #ty },
                    ),
                    AccessorKind::UnsafeConvFn { fn_, ty } => (
                        quote! { #fn_(raw_result) },
                        quote! { #ty },
                    ),

                    AccessorKind::TryGetFn { fn_, result_ty } => (
                        quote! { #fn_(raw_result) },
                        quote! { #result_ty },
                    ),
                    AccessorKind::TrySetFn { .. } => unreachable!(),
                    AccessorKind::UnwrapConvFn { fn_, ty } => (
                        quote! { #fn_(raw_result).unwrap() },
                        quote! { #ty },
                    )
                };
                let get_value = match &bits_span {
                    BitsSpan::Single(bit) => {
                        quote_spanned! {
                            ident.span() =>
                            ::proc_bitfield::__private::static_assertions::const_assert!(
                                #bit < #storage_ty_bits
                            );
                            let raw_result = <#storage_ty as ::proc_bitfield::Bit>
                                ::bit::<#bit>(&self.0);
                        }
                    }
                    BitsSpan::Range { start, end } => {
                        quote_spanned! {
                            ident.span() =>
                            ::proc_bitfield::__private::static_assertions::const_assert!(
                                #end > #start
                            );
                            ::proc_bitfield::__private::static_assertions::const_assert!(
                                #start < #storage_ty_bits && #end <= #storage_ty_bits
                            );
                            ::proc_bitfield::__private::static_assertions::const_assert!(
                                #end - #start <= #field_ty_bits
                            );
                            let raw_result = <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                ::bits::<#start, #end>(&self.0);
                        }
                    }
                    BitsSpan::Full => {
                        quote_spanned! {
                            ident.span() =>
                            let raw_result = <#storage_ty as ::proc_bitfield::Bits<#field_ty>>
                                ::bits::<0, #storage_ty_bits>(&self.0);
                        }
                    }
                };

                let get_unsafe = get_kind.is_unsafe().then(|| quote! { unsafe }).into_iter();
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis #(#get_unsafe)* fn #ident(&self) -> #get_output_ty {
                        #get_value
                        #calc_get_result
                    }
                }
            } else {
                quote! {}
            };

            let setters = if !matches!(&set_kind, AccessorKind::Disabled) {
                let (
                    calc_set_with_value,
                    set_with_input_ty,
                    set_ok,
                    set_output_ty,
                    with_ok,
                    with_output_ty,
                ) = match set_kind {
                    AccessorKind::Default => (
                        quote! { value },
                        field_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::Disabled => unreachable!(),

                    AccessorKind::ConvTy(set_ty) => (
                        quote! { <#set_ty as ::core::convert::Into<#field_ty>>::into(value) },
                        set_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::UnsafeConvTy(set_ty) => (
                        quote! {
                            <#set_ty as ::proc_bitfield::UnsafeInto<#field_ty>>::unsafe_into(value)
                        },
                        set_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::TryConvTy(set_ty) => (
                        quote! {
                            <#set_ty as ::core::convert::TryInto<#field_ty>>::try_into(value)?
                        },
                        set_ty,
                        quote! { ::core::result::Result::Ok(()) },
                        quote! {
                            ::core::result::Result<
                                (),
                                <#set_ty as ::core::convert::TryInto<#field_ty>>::Error
                            >
                        },
                        quote! { ::core::result::Result::Ok(raw_result) },
                        quote! {
                            ::core::result::Result<
                                Self,
                                <#set_ty as ::core::convert::TryInto<#field_ty>>::Error
                            >
                        },
                    ),
                    AccessorKind::UnwrapConvTy(set_ty) => (
                        quote! {
                            <#set_ty as ::core::convert::TryInto<#field_ty>>::try_into(value)
                                .unwrap()
                        },
                        set_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),

                    AccessorKind::ConvFn { fn_, ty } => (
                        quote! { #fn_(value) },
                        ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::UnsafeConvFn { fn_, ty } => (
                        quote! { #fn_(value) },
                        ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::TryGetFn { .. } => unreachable!(),
                    AccessorKind::TrySetFn { fn_, input_ty, result_ty } => (
                        quote! { #fn_(value)? },
                        input_ty,
                        quote! {
                            <#result_ty as ::proc_bitfield::Try>::WithOutput::<()>::from_output(())
                        },
                        quote! { <#result_ty as ::proc_bitfield::Try>::WithOutput<()> },
                        quote! {
                            <#result_ty as ::proc_bitfield::Try>::WithOutput::<Self>::from_output(
                                raw_result,
                            )
                        },
                        quote! { <#result_ty as ::proc_bitfield::Try>::WithOutput<Self> },
                    ),
                    AccessorKind::UnwrapConvFn { fn_, ty } => (
                        quote! { #fn_(value).unwrap() },
                        ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                };

                let with_value = match &bits_span {
                    BitsSpan::Single(bit) => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::WithBit>::with_bit::<#bit>(
                            self.0,
                            #calc_set_with_value,
                        )
                    },
                    BitsSpan::Range { start, end } => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                            ::with_bits::<#start, #end>(self.0, #calc_set_with_value)
                    },
                    BitsSpan::Full => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                            ::with_bits::<0, #storage_ty_bits>(self.0, #calc_set_with_value)
                    },
                };

                let set_value = match &bits_span {
                    BitsSpan::Single(bit) => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::SetBit>::set_bit::<#bit>(
                            &mut self.0,
                            #calc_set_with_value,
                        )
                    },
                    BitsSpan::Range { start, end } => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                            ::set_bits::<#start, #end>(&mut self.0, #calc_set_with_value)
                    },
                    BitsSpan::Full => quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                            ::set_bits::<0, #storage_ty_bits>(&mut self.0, #calc_set_with_value)
                    },
                };

                let set_with_unsafe = set_kind.is_unsafe().then(|| quote! { unsafe });
                let set_with_unsafe_1 = set_with_unsafe.iter();
                let set_with_unsafe_2 = set_with_unsafe_1.clone();
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[must_use]
                    #[allow(clippy::identity_op)]
                    #vis #(#set_with_unsafe_1)* fn #with_fn_ident(
                        self,
                        value: #set_with_input_ty,
                    ) -> #with_output_ty {
                        let raw_result = Self(#with_value);
                        #with_ok
                    }

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis #(#set_with_unsafe_2)* fn #set_fn_ident(
                        &mut self,
                        value: #set_with_input_ty,
                    ) -> #set_output_ty {
                        #set_value;
                        #set_ok
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                #getter
                #setters
            }
        },
    ).collect::<Vec<_>>();

    let mut auto_trait_impls = Vec::new();

    if auto_impls.debug {
        let readable_fields = fields
            .iter()
            .filter(|field| !matches!(field.get_kind, AccessorKind::Disabled));
        let field_idents = readable_fields.clone().map(|field| &field.ident);
        let field_unsafes = readable_fields.map(|field| {
            if field.get_kind.is_unsafe() {
                quote! { unsafe }
            } else {
                quote! {}
            }
        });
        auto_trait_impls.push(quote! {
            impl #generics ::core::fmt::Debug for #ident #where_clause {
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

    if auto_impls.from_raw {
        auto_trait_impls.push(quote! {
            impl #generics ::core::convert::From<#storage_ty> for #ident #where_clause {
                fn from(other: #storage_ty) -> Self {
                    Self(other)
                }
            }
        });
    }

    if auto_impls.into_raw {
        auto_trait_impls.push(quote! {
            impl #generics ::core::convert::From<#ident> for #storage_ty #where_clause {
                fn from(other: #ident) -> Self {
                    other.0
                }
            }
        });
    }

    if auto_impls.deref_raw {
        auto_trait_impls.push(quote! {
            impl #generics ::core::ops::Deref for #ident #where_clause {
                type Target = #storage_ty;

                fn deref(&self) -> &#storage_ty {
                    &self.0
                }
            }
        });
    }

    (quote! {
        #(#outer_attrs)*
        #vis struct #ident #generics(#storage_vis #storage_ty) #where_clause;

        impl #generics #ident #where_clause {
            #(#field_fns)*
        }

        #(#auto_trait_impls)*
    })
    .into()
}
