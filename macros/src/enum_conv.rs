use crate::utils::for_all_int_types;
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitInt, UnOp,
    Variant,
};

fn parse_discrs<'a>(
    variants: impl Iterator<Item = &'a Variant> + 'a,
) -> impl Iterator<Item = (Expr, isize)> + 'a {
    let mut next_discr_int = 0_isize;
    variants.map(move |variant| {
        if let Some((_, discr)) = &variant.discriminant {
            if let Some(discr_int) = match discr {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(lit_int),
                    ..
                }) => Some(
                    lit_int
                        .base10_parse::<isize>()
                        .expect("Couldn't parse discriminant"),
                ),
                Expr::Unary(ExprUnary {
                    op: UnOp::Neg(_),
                    expr,
                    ..
                }) => {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Int(lit_int),
                        ..
                    }) = &**expr
                    {
                        Some(
                            -lit_int
                                .base10_parse::<isize>()
                                .expect("Couldn't parse discriminant"),
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            } {
                next_discr_int = discr_int + 1;
                (discr.clone(), discr_int)
            } else {
                unimplemented!("Non-literal discriminants are unsupported");
            }
        } else {
            let discr_int = next_discr_int;
            next_discr_int += 1;
            (
                Expr::Lit(ExprLit {
                    lit: Lit::Int(LitInt::new(
                        &format!("{}", discr_int),
                        Span::call_site().into(),
                    )),
                    attrs: Vec::new(),
                }),
                discr_int,
            )
        }
    })
}

fn signed_bounds(discr_bits: u8) -> (i128, i128) {
    (
        -1_i128 << (discr_bits - 1),
        ((1_u128 << (discr_bits - 1)) - 1) as i128,
    )
}

fn unsigned_bound(discr_bits: u8) -> u128 {
    (1_u128 << (discr_bits - 1) << 1).wrapping_sub(1)
}

pub fn derive_conv_raw(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match &input.data {
        Data::Enum(data) => {
            if data.variants.is_empty() {
                unimplemented!("#[derive(ConvRaw)] requires a populated enum");
            }

            if data
                .variants
                .iter()
                .any(|variant| !matches!(variant.fields, Fields::Unit))
            {
                unimplemented!("#[derive(ConvRaw)] requires a fieldless enum");
            }

            let discr_data = parse_discrs(data.variants.iter()).collect::<Vec<_>>();
            let (min_discr, max_discr) =
                discr_data.iter().fold((0, 0), |(min, max), (_, discr)| {
                    (min.min(*discr), max.max(*discr))
                });

            let mut impls = Vec::new();

            // Implement TryFrom/UnsafeFrom<u/i8..=u/i128>
            for_all_int_types(|discr_bits, signed, discr_ty| {
                let from_raw_variants = discr_data
                    .iter()
                    .zip(&data.variants)
                    .filter(|((_, discr), _)| {
                        if signed {
                            let (min, max) = signed_bounds(discr_bits);
                            *discr as i128 >= min && *discr as i128 <= max
                        } else {
                            *discr >= 0 && *discr as u128 <= unsigned_bound(discr_bits)
                        }
                    })
                    .map(|((discr_lit, _), variant)| {
                        let variant_name = &variant.ident;
                        quote! {
                            #discr_lit => #type_name::#variant_name,
                        }
                    });
                let from_raw_variants_ = from_raw_variants.clone();
                let from_raw_impls = quote! {
                    impl #impl_generics ::core::convert::TryFrom<#discr_ty>
                        for #type_name #ty_generics
                        #where_clause
                    {
                        type Error = ();

                        fn try_from(other: #discr_ty) -> Result<#type_name #ty_generics, ()> {
                            Ok(match other {
                                #(#from_raw_variants)*
                                _ => return Err(()),
                            })
                        }
                    }

                    impl #impl_generics ::proc_bitfield::UnsafeFrom<#discr_ty>
                        for #type_name #ty_generics
                        #where_clause
                    {
                        unsafe fn unsafe_from(other: #discr_ty,) -> #type_name #ty_generics {
                            match other {
                                #(#from_raw_variants_)*
                                _ => ::core::hint::unreachable_unchecked(),
                            }
                        }
                    }
                };

                impls.push(from_raw_impls);
            });

            // Implement Into<u/i<min_discr_bits>..=u/i128>
            for_all_int_types(|discr_bits, signed, discr_ty| {
                let fits_discr_range = if signed {
                    let (min, max) = signed_bounds(discr_bits);
                    min_discr as i128 >= min && max_discr as i128 <= max
                } else {
                    min_discr >= 0 && max_discr as u128 <= unsigned_bound(discr_bits)
                };
                if !fits_discr_range {
                    return;
                }

                let into_raw_variants =
                    discr_data
                        .iter()
                        .zip(&data.variants)
                        .map(|((discr_lit, _), variant)| {
                            let variant_name = &variant.ident;
                            quote! {
                                #type_name::#variant_name => #discr_lit,
                            }
                        });
                let into_raw_impl = quote! {
                    #[allow(unused_variables)]
                    impl #impl_generics ::core::convert::From<#type_name #ty_generics> for #discr_ty
                        #where_clause
                    {
                        fn from(other: #type_name #ty_generics) -> #discr_ty {
                            match other {
                                #(#into_raw_variants)*
                            }
                        }
                    }
                };

                impls.push(into_raw_impl);
            });

            quote! { #(#impls)* }.into()
        }

        Data::Struct(_) => unimplemented!("Can't derive ConvRaw on structs"),
        Data::Union(_) => unimplemented!("Can't derive ConvRaw on unions"),
    }
}
