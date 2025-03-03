use crate::utils::for_all_int_types;
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitInt, UnOp,
    Variant,
};

fn parse_discrs<'a>(
    variants: impl Iterator<Item = &'a Variant> + 'a,
) -> impl Iterator<Item = (&'a Variant, Expr, isize)> {
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
                (variant, discr.clone(), discr_int)
            } else {
                unimplemented!("Non-literal discriminants are unsupported");
            }
        } else {
            let discr_int = next_discr_int;
            next_discr_int += 1;
            (
                variant,
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
                discr_data.iter().fold((0, 0), |(min, max), (_, _, discr)| {
                    (min.min(*discr), max.max(*discr))
                });

            let mut impls = Vec::new();

            // Implement TryFrom/UnsafeFrom<u/i8..=u/i128>
            for_all_int_types(|discr_bits, signed, discr_ty| {
                let from_raw_variants = discr_data
                    .iter()
                    .filter(|(_, _, discr)| {
                        if signed {
                            let (min, max) = signed_bounds(discr_bits);
                            *discr as i128 >= min && *discr as i128 <= max
                        } else {
                            *discr >= 0 && *discr as u128 <= unsigned_bound(discr_bits)
                        }
                    })
                    .map(|(variant, discr_lit, _)| {
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

                let into_raw_variants = discr_data.iter().map(|(variant, discr_lit, _)| {
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

            // Implement From<bool> and Into<bool>
            if let [(v_false, _, 0), (v_true, _, 1)] | [(v_true, _, 1), (v_false, _, 0)] =
                discr_data.as_slice()
            {
                let v_false_name = &v_false.ident;
                let v_true_name = &v_true.ident;

                let impl_from_bool = quote! {
                    impl #impl_generics ::core::convert::From<bool> for #type_name #ty_generics
                        #where_clause
                    {
                        fn from(other: bool) -> #type_name #ty_generics {
                            match other {
                                false => #type_name::#v_false_name,
                                true => #type_name::#v_true_name,
                            }
                        }
                    }
                };

                let impl_into_bool = quote! {
                    #[allow(unused_variables)]
                    impl #impl_generics ::core::convert::From<#type_name #ty_generics> for bool
                        #where_clause
                    {
                        fn from(other: #type_name #ty_generics) -> bool {
                            match other {
                                #type_name::#v_false_name => false,
                                #type_name::#v_true_name => true,
                            }
                        }
                    }
                };

                impls.push(impl_into_bool);
                impls.push(impl_from_bool);
            }

            quote! { #(#impls)* }.into()
        }

        Data::Struct(_) => unimplemented!("Can't derive ConvRaw on structs"),
        Data::Union(_) => unimplemented!("Can't derive ConvRaw on unions"),
    }
}
