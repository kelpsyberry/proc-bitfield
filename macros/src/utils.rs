use quote::format_ident;
use syn::{
    parenthesized,
    parse::{ParseBuffer, ParseStream},
    Ident, Result,
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
