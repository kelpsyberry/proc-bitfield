use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Error, Generics, Ident, Lit, Token, Type, Visibility, WhereClause,
};

mod kw {
    syn::custom_keyword!(get);
    syn::custom_keyword!(try_get);
    syn::custom_keyword!(unsafe_get);
    syn::custom_keyword!(set);
    syn::custom_keyword!(try_set);
    syn::custom_keyword!(read_only);
    syn::custom_keyword!(write_only);
    syn::custom_keyword!(Debug);
}

enum Bits {
    Single(Lit),
    Range { start: Lit, end: Lit },
    RangeInclusive { start: Lit, end: Lit },
    OffsetAndLength { start: Lit, length: Lit },
    RangeFull,
}

enum AccessorKind {
    None,
    Conv(Type),
    UnsafeConv(Type),
    TryConv(Type),
    Disabled,
}

struct Field {
    attrs: Vec<Attribute>,
    vis: Visibility,
    #[cfg(feature = "nightly")]
    is_const: bool,
    ident: Ident,
    ty: Type,
    get_ty: AccessorKind,
    set_ty: AccessorKind,
    bits: Bits,
}

struct Struct {
    outer_attrs: Vec<Attribute>,
    vis: Visibility,
    #[cfg(feature = "nightly")]
    is_const: bool,
    ident: Ident,
    storage_vis: Visibility,
    storage_ty: Type,
    has_debug: bool,
    generics: Generics,
    where_clause: Option<WhereClause>,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        #[cfg(feature = "nightly")]
        let is_const = input.parse::<Token![const]>().is_ok();
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        let (storage_vis, storage_ty) = {
            let content;
            parenthesized!(content in input);
            (content.parse()?, content.parse()?)
        };

        let has_debug = input.parse::<Token![:]>().is_ok();
        if has_debug {
            input.parse::<kw::Debug>()?;
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
        let fields = content.parse_terminated(|input| {
            let attrs = input.call(Attribute::parse_outer)?;
            let vis = input.parse()?;
            let ident = input.parse()?;
            input.parse::<Token![:]>()?;
            #[cfg(feature = "nightly")]
            let is_const = input.parse::<Token![const]>().is_ok();
            let ty = input.parse()?;
            let (get_ty, set_ty) = {
                let lookahead = input.lookahead1();
                if lookahead.peek(token::Bracket) {
                    let content;
                    bracketed!(content in input);
                    let mut get_ty = AccessorKind::None;
                    let mut set_ty = AccessorKind::None;
                    macro_rules! check_not_duplicated {
                        ($($ident: ident),*; $span: expr) => {
                            if $(!matches!(&$ident, AccessorKind::None))||* {
                                return Err(Error::new(
                                    $span,
                                    "Duplicated conversion type definition",
                                ));
                            }
                        };
                    }
                    while !content.is_empty() {
                        if let Ok(kw) = content.parse::<kw::get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::Conv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::try_get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::TryConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::unsafe_get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::UnsafeConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::set>() {
                            check_not_duplicated!(set_ty; kw.span);
                            set_ty = AccessorKind::Conv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::try_set>() {
                            check_not_duplicated!(set_ty; kw.span);
                            set_ty = AccessorKind::TryConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<Token![try]>() {
                            check_not_duplicated!(get_ty, set_ty; kw.span);
                            let ty: Type = content.parse()?;
                            get_ty = AccessorKind::TryConv(ty.clone());
                            set_ty = AccessorKind::TryConv(ty);
                        } else if let Ok(kw) = content.parse::<Token![unsafe]>() {
                            check_not_duplicated!(get_ty, set_ty; kw.span);
                            let ty: Type = content.parse()?;
                            get_ty = AccessorKind::UnsafeConv(ty.clone());
                            set_ty = AccessorKind::UnsafeConv(ty);
                        } else if let Ok(kw) = content.parse::<kw::read_only>() {
                            if matches!(&get_ty, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    kw.span,
                                    "Duplicated read_only and write_only specifiers",
                                ));
                            }
                            set_ty = AccessorKind::Disabled;
                        } else if let Ok(kw) = content.parse::<kw::write_only>() {
                            if matches!(&set_ty, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    kw.span,
                                    "Duplicated read_only and write_only specifiers",
                                ));
                            }
                            get_ty = AccessorKind::Disabled;
                        } else {
                            let ty: Type = content.parse()?;
                            check_not_duplicated!(get_ty, set_ty; ty.span());
                            get_ty = AccessorKind::Conv(ty.clone());
                            set_ty = AccessorKind::Conv(ty);
                        }
                        let _ = content.parse::<Token![,]>();
                    }
                    (get_ty, set_ty)
                } else {
                    (AccessorKind::None, AccessorKind::None)
                }
            };
            input.parse::<Token![@]>()?;
            Ok(Field {
                attrs,
                vis,
                #[cfg(feature = "nightly")]
                is_const,
                ident,
                ty,
                get_ty,
                set_ty,
                bits: {
                    if input.parse::<Token![..]>().is_ok() {
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
                        } else if lookahead.peek(Token![,]) || input.is_empty() {
                            Bits::Single(start)
                        } else {
                            return Err(lookahead.error());
                        }
                    }
                },
            })
        })?;

        Ok(Struct {
            outer_attrs,
            vis,
            #[cfg(feature = "nightly")]
            is_const,
            ident,
            storage_vis,
            storage_ty,
            generics,
            has_debug,
            where_clause,
            fields,
        })
    }
}

#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    let Struct {
        outer_attrs,
        vis,
        #[cfg(feature = "nightly")]
        is_const: struct_is_const,
        ident,
        storage_vis,
        storage_ty,
        has_debug,
        generics,
        where_clause,
        fields,
    } = match syn::parse(input) {
        Ok(res) => res,
        Err(err) => return err.into_compile_error().into(),
    };
    let field_fns = fields.iter().map(
        |Field {
             attrs,
             vis,
             #[cfg(feature = "nightly")]
             is_const,
             ident,
             ty,
             get_ty,
             set_ty,
             bits,
         }| {
            let storage_ty_bits = quote! { (::core::mem::size_of::<#storage_ty>() << 3) };
            let ty_bits = quote! { (::core::mem::size_of::<#ty>() << 3) };
            let set_fn_ident = format_ident!("set_{}", ident);
            let with_fn_ident = format_ident!("with_{}", ident);
            #[cfg(feature = "nightly")]
            let const_token = if *is_const || struct_is_const {
                quote! { const }
            } else {
                quote! {}
            };
            #[cfg(not(feature = "nightly"))]
            let const_token = quote! {};
            let (start, end) = match bits {
                Bits::Single(bit) => (quote! { #bit }, None),
                Bits::Range { start, end } => (quote! { #start }, Some(quote! { #end })),
                Bits::RangeInclusive { start, end } => (
                    quote! { #start },
                    Some(quote! { {#end + 1} }),
                ),
                Bits::OffsetAndLength { start, length } => (
                    quote! { #start },
                    Some(quote! { {#start + #length} }),
                ),
                Bits::RangeFull => (
                    quote! { 0 },
                    Some(quote!{ { ::core::mem::size_of::<#ty>() << 3 } }),
                ),
            };

            let getter = if !matches!(&get_ty, AccessorKind::Disabled) {
                let (calc_get_result, get_output_ty) = match get_ty {
                    AccessorKind::None => (quote! { raw_result }, quote! { #ty }),
                    AccessorKind::Disabled => unreachable!(),
                    AccessorKind::Conv(get_ty) => {
                        (
                            quote! { <#get_ty as ::core::convert::From<#ty>>::from(raw_result) },
                            quote! { #get_ty },
                        )
                    }
                    AccessorKind::UnsafeConv(get_ty) => {
                        (
                            quote! { unsafe {
                                <#get_ty as ::proc_bitfield::UnsafeFrom<#ty>>::unsafe_from(
                                    raw_result,
                                )
                            } },
                            quote! { #get_ty },
                        )
                    }
                    AccessorKind::TryConv(get_ty) => (
                        quote! { #get_ty::try_from(raw_result) },
                        quote! {
                            Result<
                                #get_ty,
                                <#get_ty as ::core::convert::TryFrom<#ty>>::Error,
                            >
                        },
                    ),
                };
                let get_value = if let Some(end) = &end {
                    quote_spanned! {
                        ident.span() =>
                        ::proc_bitfield::static_assertions::const_assert!(
                            #end > #start
                        );
                        ::proc_bitfield::static_assertions::const_assert!(
                            #start < #storage_ty_bits && #end <= #storage_ty_bits
                        );
                        ::proc_bitfield::static_assertions::const_assert!(
                            #end - #start <= #ty_bits
                        );
                        let raw_result = <#storage_ty as ::proc_bitfield::BitRange<#ty>>
                            ::bit_range::<#start, #end>(self.0);
                    }
                } else {
                    quote_spanned! {
                        ident.span() =>
                        ::proc_bitfield::static_assertions::const_assert!(
                            #start < #storage_ty_bits
                        );
                        let raw_result = <#storage_ty as ::proc_bitfield::Bit>
                            ::bit::<#start>(self.0);
                    }
                };
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis #const_token fn #ident(&self) -> #get_output_ty {
                        #get_value
                        #calc_get_result
                    }
                }
            } else {
                quote! {}
            };

            let setters = if !matches!(&set_ty, AccessorKind::Disabled) {
                let (
                    calc_set_with_value,
                    set_with_input_ty,
                    set_ok,
                    set_output_ty,
                    with_ok,
                    with_output_ty,
                ) = match set_ty {
                    AccessorKind::None => (
                        quote! { value },
                        ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::Disabled => unreachable!(),
                    AccessorKind::Conv(set_ty) | AccessorKind::UnsafeConv(set_ty) => (
                        quote! { <#set_ty as ::core::convert::Into<#ty>>::into(value) },
                        set_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::TryConv(set_ty) => (
                        quote! { #set_ty::try_into(value)? },
                        set_ty,
                        quote! { Ok(()) },
                        quote! { Result<(), <#set_ty as ::core::convert::TryInto<#ty>>::Error> },
                        quote! { Ok(raw_result) },
                        quote! { Result<Self, <#set_ty as ::core::convert::TryInto<#ty>>::Error> },
                    ),
                };
                let with_value = if let Some(end) = &end {
                    quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::BitRange<#ty>>
                            ::set_bit_range::<#start, #end>(self.0, #calc_set_with_value)
                    }
                } else {
                    quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::proc_bitfield::Bit>::set_bit::<#start>(
                            self.0,
                            #calc_set_with_value,
                        )
                    }
                };
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[must_use]
                    #[allow(clippy::identity_op)]
                    #vis #const_token fn #with_fn_ident(
                        self,
                        value: #set_with_input_ty,
                    ) -> #with_output_ty {
                        let raw_result = Self(#with_value);
                        #with_ok
                    }

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis #const_token fn #set_fn_ident(
                        &mut self,
                        value: #set_with_input_ty,
                    ) -> #set_output_ty {
                        self.0 = #with_value;
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
    let impl_debug = if has_debug {
        let field_idents = fields.iter().map(|field| &field.ident);
        Some(quote! {
            impl #generics ::core::fmt::Debug for #ident #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    f.debug_struct(stringify!(#ident))
                        .field("0", &self.0)
                        #(.field(stringify!(#field_idents), &self.#field_idents()))*
                        .finish()
                }
            }
        })
    } else {
        None
    };
    (quote! {
        #(#outer_attrs)*
        #vis struct #ident #generics(#storage_vis #storage_ty) #where_clause;

        impl #generics #ident #where_clause {
            #(#field_fns)*
        }

        #impl_debug
    })
    .into()
}
