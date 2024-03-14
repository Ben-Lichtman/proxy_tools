use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, FnArg, ForeignItemFn, ItemFn, LitStr};

struct Attrs {
	lib: LitStr,
	before: Option<String>,
	after: Option<String>,
}

#[proc_macro_attribute]
pub fn gen_proxy(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let mut lib: Option<LitStr> = None;
	let mut before: Option<LitStr> = None;
	let mut after: Option<LitStr> = None;
	let meta_parser = syn::meta::parser(|meta| {
		if meta.path.is_ident("lib") {
			lib = Some(meta.value()?.parse()?);
		}
		else if meta.path.is_ident("before") {
			before = Some(meta.value()?.parse()?);
		}
		else if meta.path.is_ident("after") {
			after = Some(meta.value()?.parse()?);
		}
		else {
			return Err(meta.error("unsupported property"));
		}

		Ok(())
	});

	parse_macro_input!(attr with meta_parser);

	let attrs = Attrs {
		lib: lib.expect("library name must be provided"),
		before: before.map(|l| l.value()),
		after: after.map(|l| l.value()),
	};

	let derive_input = parse_macro_input!(item as ForeignItemFn);
	proc_macro::TokenStream::from(process_input(&attrs, &derive_input))
}

fn process_input(attrs: &Attrs, input: &ForeignItemFn) -> TokenStream {
	let Attrs { lib, before, after } = attrs;

	let ForeignItemFn {
		attrs: fn_attrs,
		vis,
		sig,
		..
	} = input;

	let sig_inputs = &sig.inputs;
	let sig_output = &sig.output;
	let num_args = sig_inputs.len();

	let args_assignments = sig_inputs
		.iter()
		.enumerate()
		.map(|(index, input)| {
			let input = match input {
				FnArg::Receiver(_) => quote! { self },
				FnArg::Typed(typed) => {
					let pat = &typed.pat;
					quote! { #pat }
				}
			};
			let ident = format_ident!("__input_{}", index);
			quote! { let mut #ident = #input; }
		})
		.collect::<Vec<_>>();

	let arg_idents = (0..num_args)
		.map(|index| format_ident!("__input_{}", index))
		.collect::<Vec<_>>();

	let before = before
		.as_deref()
		.map(|before_fn| {
			let before_ident = format_ident!("{before_fn}");
			quote! {
				#before_ident ( #( &mut #arg_idents ),* );
			}
		})
		.unwrap_or_default();

	let after = after
		.as_deref()
		.map(|after_fn| {
			let after_ident = format_ident!("{after_fn}");
			quote! {
				#after_ident ( __output );
			}
		})
		.unwrap_or_default();

	let internal_name = format_ident!("{}_internal", sig.ident);
	let external_name = format!("{}_external", sig.ident);

	let item_fn = ItemFn {
		attrs: fn_attrs.clone(),
		vis: vis.clone(),
		sig: sig.clone(),
		block: parse_quote! {{
			#[link(name = #lib, kind = "raw-dylib")]
			extern "C" {
				#[link_name = #external_name]
				fn #internal_name ( #sig_inputs ) #sig_output;
			}

			#( #args_assignments )*
			#before
			let __output = unsafe { #internal_name ( #( #arg_idents ),* ) };
			#after
			__output
		}},
	};

	quote! {
		#item_fn
	}
}
