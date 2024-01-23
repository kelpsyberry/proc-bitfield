use crate::utils::for_all_int_types;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let where_clause_start = if let Some(where_clause) = where_clause {
        quote!(#where_clause, )
    } else {
        quote!(where)
    };

    let mut impls = Vec::new();

    for_all_int_types(|bits, _, int_ty| {
        let bits = bits as usize;

        let where_clause = quote! {
            #where_clause_start #type_name #ty_generics: ::core::convert::TryFrom<#int_ty>
                + ::core::convert::Into<#int_ty>,
            <#type_name #ty_generics as ::core::convert::TryFrom<#int_ty>>::Error:
                ::core::fmt::Debug
        };

        impls.push(quote! {
            impl #impl_generics ::proc_bitfield::Bits<#type_name #ty_generics> for #int_ty
                #where_clause
            {
                #[inline]
                fn bits<const START: usize, const END: usize>(
                    &self
                ) -> #type_name #ty_generics {
                    let value = self << (#bits - END) >> (#bits - (END - START));
                    <#type_name #ty_generics as ::core::convert::TryFrom::<#int_ty>>::try_from(
                        value,
                    ).unwrap()
                }
            }

            impl #impl_generics ::proc_bitfield::WithBits<#type_name #ty_generics> for #int_ty
                #where_clause
            {
                #[inline]
                fn with_bits<const START: usize, const END: usize>(
                    self,
                    value: #type_name #ty_generics
                ) -> Self {
                    let written_bits = END - START;
                    let mask = ((1 as #int_ty) << (written_bits - 1) << 1).wrapping_sub(1) << START;
                    (self & !mask)
                        | (<#type_name #ty_generics as ::core::convert::Into::<#int_ty>>::into(
                            value,
                        ) << START & mask)
                }
            }

            impl #impl_generics ::proc_bitfield::SetBits<#type_name #ty_generics> for #int_ty
                #where_clause
            {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(
                    &mut self,
                    value: #type_name #ty_generics
                ) {
                    *self = ::proc_bitfield::WithBits::with_bits::<START, END>(*self, value);
                }
            }
        });
    });

    quote! { #(#impls)* }.into()
}
