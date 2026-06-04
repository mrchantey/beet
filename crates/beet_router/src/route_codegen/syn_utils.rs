//! Shared `syn` helpers for parsing handler signatures.

use syn::ItemFn;
use syn::Type;

/// Returns the single generic argument of a type path whose last segment
/// matches `outer`, ie `T` for `Outer<T>`.
pub(crate) fn inner_generic(ty: &Type, outer: &str) -> Option<Type> {
	let Type::Path(type_path) = ty else {
		return None;
	};
	let seg = type_path.path.segments.last()?;
	if seg.ident != outer {
		return None;
	}
	let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
		return None;
	};
	args.args.iter().find_map(|arg| match arg {
		syn::GenericArgument::Type(ty) => Some(ty.clone()),
		_ => None,
	})
}

/// Returns the last path-segment identifier of a type, ie `Json` for
/// `beet::prelude::Json<T>`.
pub(crate) fn type_last_ident(ty: &Type) -> Option<String> {
	let Type::Path(type_path) = ty else {
		return None;
	};
	type_path
		.path
		.segments
		.last()
		.map(|seg| seg.ident.to_string())
}

/// The action's input type, ie the `In` of `ActionContext<In>` in the handler's
/// first parameter (peeling a leading `In<..>` system wrapper).
pub(crate) fn action_input_ty(item: &ItemFn) -> Option<Type> {
	let syn::FnArg::Typed(pat_type) = item.sig.inputs.first()? else {
		return None;
	};
	// peel the system `In<..>` wrapper if present
	let context = inner_generic(&pat_type.ty, "In")
		.unwrap_or_else(|| (*pat_type.ty).clone());
	inner_generic(&context, "ActionContext")
}

/// The action's success output, unwrapping a single `Result<T, ..>`.
pub(crate) fn action_output_ty(item: &ItemFn) -> Type {
	let syn::ReturnType::Type(_, ty) = &item.sig.output else {
		return syn::parse_quote!(());
	};
	inner_generic(ty, "Result").unwrap_or_else(|| (**ty).clone())
}
