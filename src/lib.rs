//! # mpi_derive
//! Provide a derive macro for the trait `mpi::datatype::traits::Equivalence`.
//!
//! The macro works only for plain structures composed recursively of of:
//! - types that implement the `Equivalence` trait
//! - arrays of those types
//! - tuples of those types
//!
//! Type aliases cannot be supported, as they are defined outside of the derived type.
//!
//! `enum`s are not supported yet, `union`s may never be.

extern crate proc_macro;

use proc_macro2::{Ident, TokenStream};
use syn::{Data, DeriveInput, Fields, Index, parse_macro_input};
use syn::spanned::Spanned;

use quote::{quote, quote_spanned};

#[proc_macro_derive(Equivalence)]
pub fn derive_equivalence(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Generate the expression defining the MPI Datatype of the whole structure
    let datatype = create_struct_datatype(&name, &input.data);

    // Implement the Equivalence trait
    let expanded = quote! {
        unsafe impl #impl_generics mpi::datatype::traits::Equivalence for #name #ty_generics #where_clause {
            type Out = mpi::datatype::UserDatatype;
            fn equivalent_datatype() -> Self::Out {
                #datatype
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

/// Create a MPI Datatype for a structure
fn create_struct_datatype(struct_name: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let len = fields.named.len() as i32;
                    let offsets = fields.named.iter().map(|f| {
                        let field_name = f.ident.as_ref().unwrap();
                        let offset = offset_of_field(quote!(#struct_name), quote!(#field_name), f.span());
                        quote_spanned! { f.span() => #offset as mpi::Address }
                    });
                    let types = fields.named.iter().map(|f| {
                        get_datatype(&f.ty)
                    });
                    quote! {
                        mpi::datatype::UserDatatype::structured(
                            #len,
                            &[1; #len as usize],
                            &[#(#offsets,)*],
                            &[#(#types,)*],
                        )
                    }
                },
                Fields::Unnamed(ref fields) => {
                    let len = fields.unnamed.len() as i32;
                    let offsets = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let field_index = Index::from(i);
                        let offset = offset_of_field(quote!(#struct_name), quote!(#field_index), f.span());
                        quote_spanned! { f.span() => #offset as mpi::Address }
                    });
                    let types = fields.unnamed.iter().map(|f| {
                        get_datatype(&f.ty)
                    });
                    quote! {
                        mpi::datatype::UserDatatype::structured(
                            #len,
                            &[1; #len as usize],
                            &[#(#offsets,)*],
                            &[#(#types,)*],
                        )
                    }
                },
                Fields::Unit => unimplemented!()
            }
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!("Enums and unions are not implemented yet"),
    }
}

/// Get the MPI Datatype of types implementing the Equivalence trait, or create a MPI Datatype for arrays and tuples of those types
fn get_datatype(t: &syn::Type) -> TokenStream {
    match t {
        // Recursion for arrays
        syn::Type::Array(array) => {
            let len = &array.len;
            let element_userdatatype = get_datatype(array.elem.as_ref());
            quote_spanned! { array.span() =>
                &mpi::datatype::UserDatatype::contiguous(#len, #element_userdatatype)
            }
        }
        // Recursion for tuples
        syn::Type::Tuple(tuple) => {
            let len = tuple.elems.len() as i32;
            let offsets = tuple.elems.iter().enumerate().map(|(i, f)| {
                let field_index = Index::from(i);
                let offset = offset_of_field(quote!(#tuple), quote!(#field_index), f.span());
                quote_spanned! { f.span() =>
                    #offset as mpi::Address
                }
            });
            let types = tuple.elems.iter().map(|t| {
                get_datatype(t)
            });
            quote_spanned! { tuple.span() =>
                  &mpi::datatype::UserDatatype::structured(
                      #len,
                      &[1; #len as usize],
                      &[#(#offsets,)*],
                      &[#(#types,)*])
             }
        }
        // Real types must implement the Equivalent traits
        syn::Type::Path(path) => {
            quote_spanned! { path.span() =>
                &<#path as mpi::datatype::Equivalence>::equivalent_datatype()
            }
        }
        //_ => unimplemented!("Unimplemented for type: {:?}", t)
        _ => unimplemented!()
    }
}

/// Generate code to calculate the offset of a field in a structure
///
/// This function is equivalent to the offset_of! macro:
/// ```
/// macro_rules! offset_of {
///     ($T:ty, $field:tt) => {{
///         let value: $T = unsafe { ::std::mem::uninitialized() };
///
///         let value_loc = &value as *const _ as usize;
///         let field_loc = &value.$field as *const _ as usize;
///
///         ::std::mem::forget(value);
///
///         field_loc - value_loc
///     }};
/// }
/// ```
fn offset_of_field(type_name: TokenStream, field_name: TokenStream, span: proc_macro2::Span) -> TokenStream {
    quote_spanned! {
        span => {
             let value: #type_name = unsafe { ::std::mem::uninitialized() };

             let value_loc = &value as *const _ as usize;
             let field_loc = &value.#field_name as *const _ as usize;

             ::std::mem::forget(value);

             field_loc - value_loc
        }
    }
}
