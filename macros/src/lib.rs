//! TODO doc

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

/// TODO doc
#[proc_macro_derive(FromRow)]
pub fn from_row(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;
	let syn::Data::Struct(s) = input.data else {
		panic!("this macro only applies to structures");
	};
	let syn::Fields::Named(fields) = s.fields else {
		// TODO
		panic!("");
	};

	let fields: Vec<_> = fields
		.named
		.into_iter()
		.filter_map(|field| {
			let ident = field.ident?;
			let ident_str = format!("{ident}");

			Some(quote! {
				#ident: row.get(#ident_str)
			})
		})
		.collect();

	quote! {
		impl crate::util::FromRow for #ident {
			fn from_row(row: &tokio_postgres::Row) -> Self {
				Self {
					#(#fields),*
				}
			}
		}
	}
	.into()
}
