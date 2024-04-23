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
    token, Attribute, Error, Expr, ExprParen, ExprPath, Generics, Ident, Lit, LitInt, Token, Type,
    Visibility,
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
}

impl Field {
    fn is_readable(&self) -> bool {
        match &self.content {
            FieldContent::Single(content) => !matches!(content.get_kind, AccessorKind::Disabled),
            FieldContent::Nested(content) => content.is_readable,
        }
    }

    fn has_unsafe_getter(&self) -> bool {
        match &self.content {
            FieldContent::Single(content) => content.get_kind.is_unsafe(),
            FieldContent::Nested(_) => false,
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
    storage_vis: Visibility,
    storage_ty: Type,
    auto_impls: AutoImpls,
    generics: Generics,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let mut generics = input.parse::<Generics>()?;

        let (storage_vis, storage_ty) = {
            let content;
            parenthesized!(content in input);
            (content.parse()?, content.parse()?)
        };

        let mut auto_impls = AutoImpls {
            debug: false,
            from_storage: false,
            into_storage: false,
            deref_storage: false,
        };
        if input.parse::<Token![:]>().is_ok() {
            loop {
                if input.is_empty() {
                    break;
                }
                if input.parse::<kw::Debug>().is_ok() {
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
                let is_nested = input.parse::<kw::nested>().is_ok();
                let ty = input.parse::<Type>()?;

                let content = if is_nested {
                    let mut is_readable = true;
                    let mut is_writable = true;
                    let lookahead = input.lookahead1();
                    if lookahead.peek(token::Bracket) {
                        let options_content;
                        bracketed!(options_content in input);

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
                            if lookahead.peek(kw::read_only) || lookahead.peek(kw::ro) {
                                let span = options_content
                                    .parse::<kw::read_only>()
                                    .map(|kw| kw.span)
                                    .or_else(|_| {
                                        options_content.parse::<kw::ro>().map(|kw| kw.span)
                                    })?;
                                check_accessor_conflict!(
                                    is_writable,
                                    "read_only",
                                    is_readable,
                                    span
                                );
                                is_writable = false;
                            } else if lookahead.peek(kw::write_only) || lookahead.peek(kw::wo) {
                                let span = options_content
                                    .parse::<kw::write_only>()
                                    .map(|kw| kw.span)
                                    .or_else(|_| {
                                        options_content.parse::<kw::wo>().map(|kw| kw.span)
                                    })?;
                                check_accessor_conflict!(
                                    is_readable,
                                    "write_only",
                                    is_writable,
                                    span
                                );
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
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
                                get = AccessorKind::UnsafeConvTy {
                                    ty: options_content.parse()?,
                                    has_safe_accessor,
                                };
                            } else if let Ok(kw) = options_content.parse::<kw::unsafe_set>() {
                                check_conversion_ty_conflict!(set; kw.span);
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
                                set = AccessorKind::UnsafeConvTy {
                                    ty: options_content.parse()?,
                                    has_safe_accessor,
                                };
                            } else if let Ok(kw) = options_content.parse::<kw::unsafe_both>() {
                                check_conversion_ty_conflict!(get, set; kw.span);
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
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
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
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
                                let ty = parse_return_ty(&options_content)?
                                    .unwrap_or_else(|_| ty.clone());
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
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
                                let fn_ = parse_accessor_fn(&options_content)?;
                                let ty = parse_return_ty(&options_content)?
                                    .unwrap_or_else(|_| ty.clone());
                                get = AccessorKind::UnsafeConvFn {
                                    fn_,
                                    ty,
                                    has_safe_accessor,
                                };
                            } else if let Ok(kw) = options_content.parse::<kw::unsafe_set_fn>() {
                                check_conversion_ty_conflict!(set; kw.span);
                                let has_safe_accessor =
                                    options_content.parse::<Token![!]>().is_ok();
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
                                let ty = parse_return_ty(&options_content)?
                                    .unwrap_or_else(|_| ty.clone());
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
        fields,
    } = syn::parse_macro_input!(input);
    
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let has_type_params = generics.type_params().next().is_some();

    let type_params_phantom_data = if has_type_params {
        let type_params = generics.type_params();
        quote! { , ::core::marker::PhantomData::<#(#type_params),*> }
    } else {
        quote! {}
    };

    let field_fns = fields.iter().map(
        |Field {
             attrs,
             vis,
             ident,
             bits,
             ty: field_ty,
             content
         }| {
            let storage_ty_bits = quote! { {::core::mem::size_of::<#storage_ty>() << 3} };
            let field_ty_bits = quote! { {::core::mem::size_of::<#field_ty>() << 3} };
            let bits_span = bits.clone().into_span();
            let bits_span_asserts = match &bits_span {
                BitsSpan::Single(bit) => {
                    quote_spanned! {
                        ident.span() =>
                        ::proc_bitfield::__private::static_assertions::const_assert!(
                            #bit < #storage_ty_bits
                        );
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
                    }
                }
                BitsSpan::Full => {
                    quote! {}
                }
            };

            match content {
                FieldContent::Single(SingleField {
                    get_kind,
                    set_kind,
                }) => {
                    let set_fn_ident = format_ident!("set_{}", ident);
                    let with_fn_ident = format_ident!("with_{}", ident);

                    let getter = if !matches!(&get_kind, AccessorKind::Disabled) {
                        let (calc_get_result, get_output_ty) = match get_kind {
                            AccessorKind::Default => (quote! { raw_value }, quote! { #field_ty }),
                            AccessorKind::Disabled => unreachable!(),

                            AccessorKind::ConvTy(ty) => (
                                quote! {
                                    <#ty as ::core::convert::From<#field_ty>>::from(raw_value)
                                },
                                quote! { #ty },
                            ),
                            AccessorKind::UnsafeConvTy { ty, has_safe_accessor } => {
                                let unsafe_ = has_safe_accessor.then(|| quote! { unsafe })
                                    .into_iter();
                                (
                                    quote! {
                                        #(#unsafe_)* {
                                            <
                                                #ty as ::proc_bitfield::UnsafeFrom<#field_ty>
                                            >::unsafe_from(raw_value)
                                        }
                                    },
                                    quote! { #ty },
                                )
                            },
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
                                quote! { #ty },
                            ),

                            AccessorKind::ConvFn { fn_, ty } => (
                                quote! { #fn_(raw_value) },
                                quote! { #ty },
                            ),
                            AccessorKind::UnsafeConvFn { fn_, ty, has_safe_accessor } => {
                                let unsafe_ = has_safe_accessor.then(|| quote! { unsafe })
                                    .into_iter();
                                (
                                    quote! { #(#unsafe_)* { #fn_(raw_value) } },
                                    quote! { #ty },
                                )
                            },

                            AccessorKind::TryGetFn { fn_, result_ty } => (
                                quote! { #fn_(raw_value) },
                                quote! { #result_ty },
                            ),
                            AccessorKind::TrySetFn { .. } => unreachable!(),
                            AccessorKind::UnwrapConvFn { fn_, ty } => (
                                quote! { #fn_(raw_value).unwrap() },
                                quote! { #ty },
                            )
                        };

                        let get_raw_value = match &bits_span {
                            BitsSpan::Single(bit) => {
                                quote_spanned! {
                                    ident.span() =>
                                    let raw_value = <#storage_ty as ::proc_bitfield::Bit>
                                        ::bit::<#bit>(&self.0);
                                }
                            }
                            BitsSpan::Range { start, end } => {
                                quote_spanned! {
                                    ident.span() =>
                                    let raw_value = <
                                        #storage_ty as ::proc_bitfield::Bits<#field_ty>
                                    >::bits::<#start, #end>(&self.0);
                                }
                            }
                            BitsSpan::Full => {
                                quote_spanned! {
                                    ident.span() =>
                                    let raw_value = <
                                        #storage_ty as ::proc_bitfield::Bits<#field_ty>
                                    >::bits::<0, #storage_ty_bits>(&self.0);
                                }
                            }
                        };

                        let get_unsafe = get_kind.is_unsafe()
                            .then(|| quote! { unsafe })
                            .into_iter();
                        quote! {
                            #(#attrs)*
                            #[inline]
                            #[allow(clippy::identity_op)]
                            #vis #(#get_unsafe)* fn #ident(&self) -> #get_output_ty {
                                #bits_span_asserts
                                #get_raw_value
                                #calc_get_result
                            }
                        }
                    } else {
                        quote! {}
                    };

                    let setters = if !matches!(&set_kind, AccessorKind::Disabled) {
                        let (
                            calc_set_with_raw_value,
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

                            AccessorKind::ConvTy(ty) => (
                                quote! { <#ty as ::core::convert::Into<#field_ty>>::into(value) },
                                ty,
                                quote! {},
                                quote! { () },
                                quote! { raw_result },
                                quote! { Self },
                            ),
                            AccessorKind::UnsafeConvTy { ty, has_safe_accessor } => {
                                let unsafe_ = has_safe_accessor.then(|| quote! { unsafe })
                                    .into_iter();
                                (
                                    quote! {
                                        #(#unsafe_)* {
                                            <
                                                #ty as ::proc_bitfield::UnsafeInto<#field_ty>
                                            >::unsafe_into(value)
                                        }
                                    },
                                    ty,
                                    quote! {},
                                    quote! { () },
                                    quote! { raw_result },
                                    quote! { Self },
                                )
                            },
                            AccessorKind::TryConvTy(ty) => (
                                quote! {
                                    <#ty as ::core::convert::TryInto<#field_ty>>::try_into(value)?
                                },
                                ty,
                                quote! { ::core::result::Result::Ok(()) },
                                quote! {
                                    ::core::result::Result<
                                        (),
                                        <#ty as ::core::convert::TryInto<#field_ty>>::Error
                                    >
                                },
                                quote! { ::core::result::Result::Ok(raw_result) },
                                quote! {
                                    ::core::result::Result<
                                        Self,
                                        <#ty as ::core::convert::TryInto<#field_ty>>::Error
                                    >
                                },
                            ),
                            AccessorKind::UnwrapConvTy(ty) => (
                                quote! {
                                    <#ty as ::core::convert::TryInto<#field_ty>>::try_into(value)
                                        .unwrap()
                                },
                                ty,
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
                            AccessorKind::UnsafeConvFn { fn_, ty, has_safe_accessor } => {
                                let unsafe_ = has_safe_accessor.then(|| quote! { unsafe })
                                    .into_iter();
                                (
                                    quote! { #(#unsafe_)* { #fn_(value) } },
                                    ty,
                                    quote! {},
                                    quote! { () },
                                    quote! { raw_result },
                                    quote! { Self },
                                )
                            },
                            AccessorKind::TryGetFn { .. } => unreachable!(),
                            AccessorKind::TrySetFn { fn_, input_ty, result_ty } => (
                                quote! { #fn_(value)? },
                                input_ty,
                                quote! {
                                    <
                                        #result_ty as ::proc_bitfield::Try
                                    >::WithOutput::<()>::from_output(())
                                },
                                quote! { <#result_ty as ::proc_bitfield::Try>::WithOutput<()> },
                                quote! {
                                    <
                                        #result_ty as ::proc_bitfield::Try
                                    >::WithOutput::<Self>::from_output(raw_result)
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

                        let with_raw_value = match &bits_span {
                            BitsSpan::Single(bit) => quote_spanned! {
                                ident.span() =>
                                Self(<#storage_ty as ::proc_bitfield::WithBit>::with_bit::<#bit>(
                                    self.0,
                                    #calc_set_with_raw_value,
                                ) #type_params_phantom_data)
                            },
                            BitsSpan::Range { start, end } => quote_spanned! {
                                ident.span() =>
                                Self(<#storage_ty as ::proc_bitfield::WithBits<#field_ty>>
                                    ::with_bits::<#start, #end>(self.0, #calc_set_with_raw_value)
                                    #type_params_phantom_data
                                )
                            },
                            BitsSpan::Full => quote_spanned! {
                                ident.span() =>
                                Self(
                                    <
                                        #storage_ty as ::proc_bitfield::WithBits<#field_ty>
                                    >::with_bits::<0, #storage_ty_bits>(
                                        self.0,
                                        #calc_set_with_raw_value,
                                    )
                                    #type_params_phantom_data
                                )
                            },
                        };

                        let set_raw_value = match &bits_span {
                            BitsSpan::Single(bit) => quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::SetBit>::set_bit::<#bit>(
                                    &mut self.0,
                                    #calc_set_with_raw_value,
                                )
                            },
                            BitsSpan::Range { start, end } => quote_spanned! {
                                ident.span() =>
                                <
                                    #storage_ty as ::proc_bitfield::SetBits<#field_ty>
                                >::set_bits::<#start, #end>(
                                    &mut self.0,
                                    #calc_set_with_raw_value,
                                )
                            },
                            BitsSpan::Full => quote_spanned! {
                                ident.span() =>
                                <#storage_ty as ::proc_bitfield::SetBits<#field_ty>>
                                    ::set_bits::<0, #storage_ty_bits>(
                                        &mut self.0,
                                        #calc_set_with_raw_value,
                                    )
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
                                #bits_span_asserts
                                let raw_result = #with_raw_value;
                                #with_ok
                            }

                            #(#attrs)*
                            #[inline]
                            #[allow(clippy::identity_op)]
                            #vis #(#set_with_unsafe_2)* fn #set_fn_ident(
                                &mut self,
                                value: #set_with_input_ty,
                            ) -> #set_output_ty {
                                #set_raw_value;
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

                FieldContent::Nested(NestedField { is_readable, is_writable }) => {
                    let mut_fn_ident = format_ident!("{}_mut", ident);
                    let set_fn_ident = format_ident!("set_{}", ident);
                    let with_fn_ident = format_ident!("with_{}", ident);

                    let (start, end) = match bits_span {
                        BitsSpan::Single(_) => panic!("Nested bitfields can't be single-bit"),
                        BitsSpan::Range { start, ref end } => {
                            (start, end)
                        }
                        BitsSpan::Full => {
                            (Lit::Int(LitInt::new("0", ident.span())), &storage_ty_bits)
                        }
                    };

                    let getter = if *is_readable {
                        quote! {
                            #(#attrs)*
                            #[inline]
                            #[allow(clippy::identity_op)]
                            #vis fn #ident(&self)
                                -> ::proc_bitfield::nested::NestedRef<Self, #field_ty, #start, #end>
                            {
                                #bits_span_asserts
                                ::proc_bitfield::nested::NestedRef::new(self)
                            }
                        }
                    } else {
                        quote! {}
                    };

                    let modifier = if *is_readable && *is_writable {
                        quote! {
                            #(#attrs)*
                            #[inline]
                            #[allow(clippy::identity_op)]
                            #vis fn #mut_fn_ident(&mut self)
                                -> ::proc_bitfield::nested::NestedRefMut<
                                    Self, #field_ty, #start, #end
                                >
                            {
                                #bits_span_asserts
                                ::proc_bitfield::nested::NestedRefMut::new(self)
                            }
                        }
                    } else {
                        quote! {}
                    };

                    let setters = if *is_writable {
                        quote! {
                            #(#attrs)*
                            #[inline]
                            #[must_use]
                            #[allow(clippy::identity_op)]
                            #vis fn #with_fn_ident(self, value: #field_ty) -> Self {
                                #bits_span_asserts
                                Self(
                                    <#storage_ty as ::proc_bitfield::WithBits<
                                        <#field_ty as ::proc_bitfield::Bitfield>::Storage>
                                    >::with_bits::<#start, #end>(
                                        self.0,
                                        ::proc_bitfield::Bitfield::into_storage(value),
                                    )
                                    #type_params_phantom_data
                                )
                            }

                            #(#attrs)*
                            #[inline]
                            #[allow(clippy::identity_op)]
                            #vis fn #set_fn_ident(&mut self, value: #field_ty) {
                                <#storage_ty as ::proc_bitfield::SetBits<
                                    <#field_ty as ::proc_bitfield::Bitfield>::Storage>
                                >::set_bits::<#start, #end>(
                                    &mut self.0,
                                    ::proc_bitfield::Bitfield::into_storage(value),
                                );
                            }
                        }
                    } else {
                        quote! {}
                    };

                    quote! {
                        #getter
                        #modifier
                        #setters
                    }
                },
            }
        },
    ).collect::<Vec<_>>();

    let mut impls = vec![quote! {
        impl #impl_generics ::proc_bitfield::Bitfield for #ident #ty_generics #where_clause {
            type Storage = #storage_ty;

            #[inline]
            fn from_storage(storage: Self::Storage) -> Self {
                Self(storage #type_params_phantom_data)
            }

            #[inline]
            fn into_storage(self) -> Self::Storage {
                self.0
            }

            #[inline]
            fn storage(&self) -> &Self::Storage {
                &self.0
            }

            #[inline]
            fn storage_mut(&mut self) -> &mut Self::Storage {
                &mut self.0
            }
        }
    }];

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
                    Self(other #type_params_phantom_data)
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

    let type_params_phantom_data_field = if has_type_params {
        let type_params = generics.type_params();
        quote! { , #storage_vis ::core::marker::PhantomData<(#(#type_params),*)> }
    } else {
        quote! {}
    };

    (quote! {
        #(#outer_attrs)*
        #[repr(transparent)]
        #vis struct #ident #generics(
            #storage_vis #storage_ty #type_params_phantom_data_field
        ) #where_clause;

        impl #impl_generics #ident #ty_generics #where_clause {
            #(#field_fns)*
        }

        #(#impls)*
    })
    .into()
}
