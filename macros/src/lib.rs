mod bitfield;
mod enum_conv;
#[cfg(feature = "nightly")]
mod unwrap_bitrange;
mod utils;

use proc_macro::TokenStream;

#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    bitfield::bitfield(input)
}

#[proc_macro_derive(ConvRaw)]
pub fn derive_conv_raw(item: TokenStream) -> TokenStream {
    enum_conv::derive_conv_raw(item)
}

#[cfg(feature = "nightly")]
#[proc_macro_derive(UnwrapBitRange)]
pub fn derive_unwrap_bitrange(item: TokenStream) -> TokenStream {
    unwrap_bitrange::derive(item)
}
