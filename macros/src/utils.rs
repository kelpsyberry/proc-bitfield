use std::cell::Cell;

#[cfg(feature = "gce")]
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed, parenthesized, parse::{Parse, ParseBuffer, ParseStream}, punctuated::Punctuated, Ident, Result
};
#[cfg(feature = "gce")]
use syn::{
    AttrStyle, Attribute, ConstParam, Expr, MacroDelimiter, MetaList, Type, TypeParam,
    TypeParamBound, WherePredicate,
};

pub fn for_all_int_types(mut f: impl FnMut(u8, bool, Ident)) {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    for bits in core::iter::successors(Some(8_u8), |bits| (*bits < 128).then(|| *bits << 1)) {
        for signed in [true, false] {
            let ty_ident = format_ident!("{}{}", if signed { 'i' } else { 'u' }, bits);
            f(bits, signed, ty_ident)
        }
    }
}

pub fn parse_parens(input: ParseStream<'_>) -> Result<ParseBuffer<'_>> {
    let content;
    parenthesized!(content in input);
    Ok(content)
}

pub fn parse_brackets(input: ParseStream<'_>) -> Result<ParseBuffer<'_>> {
    let content;
    bracketed!(content in input);
    Ok(content)
}

pub fn parse_braces(input: ParseStream<'_>) -> Result<ParseBuffer<'_>> {
    let content;
    braced!(content in input);
    Ok(content)
}

pub fn maybe_const_assert(is_const: bool) -> proc_macro2::TokenStream {
    if is_const {
        quote! { ::proc_bitfield::__private::static_assertions::const_assert! }
    } else {
        quote! { ::core::assert! }
    }
}

#[cfg(feature = "gce")]
pub fn type_param(
    ident: Ident,
    bounds: impl IntoIterator<Item = TypeParamBound>,
    default: Option<Type>,
) -> TypeParam {
    TypeParam {
        attrs: Vec::new(),
        ident,
        colon_token: Default::default(),
        bounds: bounds.into_iter().collect(),
        eq_token: Default::default(),
        default,
    }
}

#[cfg(feature = "gce")]
pub fn const_param(ident: Ident, ty: Type, default: Option<Expr>) -> ConstParam {
    ConstParam {
        attrs: Vec::new(),
        const_token: Default::default(),
        ident,
        colon_token: Default::default(),
        ty,
        eq_token: Default::default(),
        default,
    }
}

pub struct MaybeRepeat {
    content: proc_macro2::TokenStream,
    was_consumed: Cell<bool>,
    after_first: Option<proc_macro2::TokenStream>,
}

impl MaybeRepeat {
    pub fn new(content: proc_macro2::TokenStream, should_repeat: bool) -> Self {
        MaybeRepeat {
            content,
            was_consumed: Cell::new(false),
            after_first: should_repeat.then(Default::default),
        }
    }

    pub fn get(&self) -> &proc_macro2::TokenStream {
        if let Some(after_first) = &self.after_first {
            if self.was_consumed.replace(true) {
                return after_first;
            }
        }
        &self.content
    }
}

pub fn parse_terminated<T, P: Parse>(
    input: ParseStream,
    mut parser: impl FnMut(ParseStream) -> syn::Result<T>,
) -> syn::Result<Punctuated<T, P>> {
    let mut punctuated = Punctuated::new();
    loop {
        if input.is_empty() {
            break;
        }
        let value = parser(input)?;
        punctuated.push_value(value);
        if input.is_empty() {
            break;
        }
        let punct = input.parse()?;
        punctuated.push_punct(punct);
    }
    Ok(punctuated)
}

#[cfg(feature = "gce")]
pub fn doc_hidden() -> Attribute {
    Attribute {
        pound_token: Default::default(),
        style: AttrStyle::Outer,
        bracket_token: Default::default(),
        meta: MetaList {
            path: Ident::new("doc", Span::call_site()).into(),
            delimiter: MacroDelimiter::Paren(Default::default()),
            tokens: quote! { hidden },
        }
        .into(),
    }
}

#[cfg(feature = "gce")]
pub fn min(a: &proc_macro2::TokenStream, b: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        ::proc_bitfield::__private::min((#a), (#b))
    }
}

#[cfg(feature = "gce")]
pub fn sized_pred(value: &proc_macro2::TokenStream) -> WherePredicate {
    syn::parse(quote! { [(); {#value}]: Sized }.into()).unwrap()
}
