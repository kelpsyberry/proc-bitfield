mod bitfield;
mod bits;
mod enum_conv;
#[cfg(feature = "nightly")]
mod unwrap_bits;
mod utils;

use proc_macro::TokenStream;

#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    bitfield::bitfield(input)
}

#[proc_macro]
pub fn bits(input: TokenStream) -> TokenStream {
    bits::bits(input)
}

#[proc_macro]
pub fn with_bits(input: TokenStream) -> TokenStream {
    bits::with_bits(input)
}

#[proc_macro]
pub fn set_bits(input: TokenStream) -> TokenStream {
    bits::set_bits(input)
}

#[proc_macro_derive(ConvRaw)]
pub fn derive_conv_raw(item: TokenStream) -> TokenStream {
    enum_conv::derive_conv_raw(item)
}

#[cfg(feature = "nightly")]
#[proc_macro_derive(UnwrapBits)]
pub fn derive_unwrap_bits(item: TokenStream) -> TokenStream {
    unwrap_bits::derive(item)
}
