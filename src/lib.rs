//! Derive macro for the [`PhysAddr`] trait.
//!
//! [`PhysAddr`]: https://docs.rs/the-mm-core/latest/the_mm_core/trait.PhysAddr.html
//!
//! [`#[derive(PhysAddr)]`](derive@PhysAddr) generates a transparent
//! implementation for a single-field tuple struct wrapping another
//! [`PhysAddr`] implementor, e.g. `struct Addr(usize)`. All arithmetic is
//! delegated to the inner type, so the overflow-safety and panicking
//! guarantees of [`PhysAddr`] are preserved.
//!
//! The trait is defined in the shared `the-mm-core` crate and re-exported
//! by `the-memblock`, so the same address type works with the buddy and
//! slab allocators too.
//!
//! PFN conversions are not part of `PhysAddr`; they live in
//! `the_mm_core::PageFrame`. A custom type that needs
//! `Memblock::mem_pfn_ranges` must also implement `PageFrame` (primitive
//! types already do, and `#[derive(the_buddy_system::PageFrame)]` generates
//! it).
//!
//! In addition to the `PhysAddr` implementation, the derive generates the
//! `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Add`
//! and `Sub` implementations that the trait requires as supertraits,
//! so `#[derive(PhysAddr)] struct Addr(usize);` works out of the box.
//! `Debug` is not generated; add it to the derive list if you need to format
//! the address type.
//!
//! Only concrete, non-generic tuple structs with exactly one field are
//! supported.
//!
//! # no_std note
//!
//! A proc-macro crate always executes at compile time on the build host,
//! linked against the compiler's `proc_macro` API, and is never shipped to
//! the target. `#![no_std]` here only states that this macro's own logic
//! needs no `std` items. What matters for no_std targets is the *generated*
//! code, which is written against `::core` and is therefore no_std-clean.

#![no_std]
extern crate alloc;

use alloc::format;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Type, parse_macro_input};

/// Derives [`PhysAddr`] and its required supertrait implementations for a
/// single-field tuple struct.
///
/// [`PhysAddr`]: https://docs.rs/the-mm-core/latest/the_mm_core/trait.PhysAddr.html
#[proc_macro_derive(PhysAddr)]
pub fn derive_phys_addr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Validates the input and extracts the struct name and inner field type.
fn single_field(input: &DeriveInput) -> syn::Result<(&Ident, &Type)> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "`#[derive(PhysAddr)]` does not support generic types",
        ));
    }
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                Ok((&input.ident, &fields.unnamed[0].ty))
            }
            Fields::Unnamed(fields) => Err(syn::Error::new_spanned(
                &data.fields,
                format!(
                    "`#[derive(PhysAddr)]` requires exactly one field, found {}",
                    fields.unnamed.len()
                ),
            )),
            _ => Err(syn::Error::new_spanned(
                &data.fields,
                "`#[derive(PhysAddr)]` only supports tuple structs with a single unnamed \
                 field, e.g. `struct Addr(usize)`",
            )),
        },
        Data::Enum(data) => Err(syn::Error::new_spanned(
            data.enum_token,
            "`#[derive(PhysAddr)]` cannot be applied to an enum",
        )),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "`#[derive(PhysAddr)]` cannot be applied to a union",
        )),
    }
}

/// Resolves the path prefix that leads to the `the_memblock` crate.
fn crate_path() -> TokenStream2 {
    match crate_name("the-memblock") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::the_memblock),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let (name, inner) = single_field(input)?;
    let path = crate_path();
    let phys = quote!(#path::PhysAddr);

    Ok(quote! {
        impl Clone for #name {
            fn clone(&self) -> Self {
                #name(self.0.clone())
            }
        }

        impl Copy for #name {}

        impl PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for #name {}

        impl PartialOrd for #name {
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
            }
        }

        impl Ord for #name {
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.0, &other.0)
            }
        }

        impl ::core::ops::Add for #name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                #name(self.0.add(rhs.0))
            }
        }

        impl ::core::ops::Sub for #name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                #name(self.0.sub(rhs.0))
            }
        }

        impl #phys for #name {
            const MAX: Self = #name(<#inner as #phys>::MAX);
            const ZERO: Self = #name(<#inner as #phys>::ZERO);

            fn align_up(addr: Self, alignment: Self) -> Self {
                #name(<#inner as #phys>::align_up(addr.0, alignment.0))
            }

            fn align_down(addr: Self, alignment: Self) -> Self {
                #name(<#inner as #phys>::align_down(addr.0, alignment.0))
            }
        }
    })
}
