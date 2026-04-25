use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


/// A token is like a typed pointer for the application layer.
/// It stores a path to a document field, and a schema.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2 {
	/// Path to the value for this token
	field: FieldRef,
	schema: TokenSchema,
}


impl Token2 {
	pub fn new(field: FieldRef, schema: TokenSchema) -> Self {
		Self { field, schema }
	}

	/// Create new token, using `Token` for the field path
	pub fn of<Field: TypePath, Schema: TypePath>() -> Self {
		Self {
			field: FieldRef::of::<Field>(),
			schema: TokenSchema::of::<Schema>(),
		}
	}
	/// Create new token, using `Token` for the field path,
	/// and serializing [`Val`] for the initial value
	pub fn new_field_reflect<Token: TypePath, Val: Typed>(
		value: Val,
	) -> Result<Self> {
		let value = Value::from_reflect(&value)?;
		Self {
			field: FieldRef::of::<Token>().with_init(value).into(),
			schema: TokenSchema::of::<Val>(),
		}
		.xok()
	}
}

/// A type which represents a token, see `token2!` for defining.
pub trait TypedToken {
	fn schema() -> TokenSchema;
	fn path() -> FieldPath;
	fn field() -> FieldRef;
	fn token() -> Token2 {
		Token2 {
			field: Self::field(),
			schema: Self::schema(),
		}
	}
}

impl<T: TypedToken> From<T> for Token2 {
	fn from(_: T) -> Self { T::token() }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueOrRef {
	Value(Value),
	Ref(FieldRef),
}

impl<T: Into<Token2>> From<T> for FieldRef {
	fn from(value: T) -> Self { value.into().field }
}

impl<T: Into<Value>> From<T> for ValueOrRef {
	fn from(value: T) -> Self { Self::Value(value.into()) }
}

impl From<FieldRef> for ValueOrRef {
	fn from(field_ref: FieldRef) -> Self { Self::Ref(field_ref) }
}

/// Represents a unique id to a token, using either a [`TypePath`]
/// or user defined namspace. These can be directly mapped to a FieldId
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenSchema {
	inner: TokenSchemaInner,
}

impl TokenSchema {
	pub fn new_field(field: FieldRef) -> Self {
		Self {
			inner: TokenSchemaInner::Field(field),
		}
	}
	pub fn of<T: bevy::reflect::TypePath>() -> Self {
		Self {
			inner: TokenSchemaInner::TypePath(SmolStr::new_static(
				T::type_path(),
			)),
		}
	}
}

impl std::fmt::Display for TokenSchema {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.inner.fmt(f)
	}
}

// sealed to protect Path variant from typepath/typename confusion
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum TokenSchemaInner {
	/// For dynamic schemas, point to the field where it can be located
	Field(FieldRef),
	/// The stable bevy [`TypePath::type_path`] to the type
	/// of this instance.
	/// This is not the [`std::any::type_name`], which
	/// is unstable.
	TypePath(SmolStr),
}


/// A user defined string, in [reverse domain name format](https://en.wikipedia.org/wiki/Reverse_domain_name_notation),
/// ie `org.beet/foo/bar`
// pub struct Namespace(SmolStr);


impl std::fmt::Display for TokenSchemaInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Field(s) => write!(f, "Field({})", s),
			Self::TypePath(s) => write!(f, "TypePath({})", s),
		}
	}
}


/// An token map is like a [`Value`] where branch nodes
/// are nested maps and leaf nodes are typed values.
/// It is perhaps more akin to a filesystem where files are
/// typed, than a freeform json value.
#[derive(Default, Deref)]
pub struct TokenMap2 {
	tokens: HashMap<FieldPath, Token2>,
}
impl TokenMap2 {
	pub fn new() -> Self {
		Self {
			tokens: HashMap::new(),
		}
	}
	/// ## Errors
	///
	/// Errors if an existing path exists anywhere up this paths chain,
	/// which would result in overlapping schemas
	pub fn insert(
		&mut self,
		path: FieldPath,
		token: Token2,
	) -> Result<&mut Self> {
		// check for overlapping paths
		for i in 1..=path.len() {
			let sub_path = FieldPath::from(&path[..i]);
			if self.tokens.contains_key(&sub_path) {
				bevybail!(
					"Path {} overlaps with existing path {}",
					path,
					sub_path
				);
			}
		}

		self.tokens.insert(path, token);
		Ok(self)
	}

	/// Convert the TokenMap to a Document, resolving FieldRef values
	pub fn resolve(
		&self,
		entity: Entity,
		document_query: &mut DocumentQuery,
	) -> Result<Document> {
		let mut doc = Document::default();
		for (path, token) in self.tokens.iter() {
			document_query
				.with_field(entity, &token.field, |value| {
					doc.insert(&path, &value)
				})
				.flatten()?;
		}
		doc.xok()
	}
}



#[macro_export]
macro_rules! token2 {
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ident
	) => {
		token2!(
			$(#[$meta])* $new_ty,
			$schema_ty,
			$crate::prelude::DocumentPath::default()
		);
	};
	(
		$(#[$meta:meta])*
		$new_ty:ident,
		$schema_ty:ident,
		$doc_path: expr
	) => {
		$(#[$meta])*
		pub struct $new_ty;
		impl $crate::prelude::TypedToken for $new_ty {
			fn schema() -> $crate::prelude::TokenSchema {
				$crate::prelude::TokenSchema::of::<$schema_ty>()
			}
			fn path() -> $crate::prelude::FieldPath {
				let path = ::core::concat!(
					::core::concat!(::core::module_path!(), "::"),
					::core::stringify!($new_ty)
				);
				$crate::prelude::FieldPath::from_module_path(path)
			}
			fn field() -> $crate::prelude::FieldRef {
				$crate::prelude::FieldRef::new(Self::path())
				.with_document($doc_path)
			}
		}
	};
}



#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_name() {
		// Name::type_info().type_path().xprintln();
		let _inst = TokenSchema::of::<Name>();
		Foo::path()
			.to_string()
			.xpect_eq("beet_node.document.token2.tests.Foo");
	}

	token2!(
			/// Some cool type
			/// This now works perfectly!
			#[derive(Debug, Clone)]
			#[allow(unused)]
			Foo,
			Color,
			DocumentPath::Ancestor
	);
	token2!(
		#[allow(unused)]
		Bar,
		Color
	);
	token2!(
		#[allow(unused)]
		Boo,
		Color
	);
}
