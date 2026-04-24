use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

#[derive(Debug, Clone, Reflect, Get)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2 {
	/// The value or fieldref for this token, matching its
	/// associated schema.
	value: ValueOrRef,
	/// Schema for the value of this instance.
	/// This may be a reference to an external schema,
	/// which must be available for validation.
	schema: TokenSchema,
}


impl Token2 {
	pub fn new(value: impl Into<ValueOrRef>, schema: TokenSchema) -> Self {
		Self {
			value: value.into(),
			schema,
		}
	}

	pub fn new_value<T: Typed>(value: T) -> Result<Self> {
		let value = Value::from_reflect(&value)?;
		Self {
			value: ValueOrRef::Value(value),
			schema: TokenSchema::of::<T>(),
		}
		.xok()
	}
	/// Create new token, using `Token` for the field path
	pub fn new_field<Token: TypePath, Schema: TypePath>() -> Self {
		Self {
			value: FieldRef::of::<Token>().into(),
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
			value: FieldRef::of::<Token>().with_init(value).into(),
			schema: TokenSchema::of::<Val>(),
		}
		.xok()
	}
}

#[derive(Debug, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueOrRef {
	Value(Value),
	Ref(FieldRef),
}

impl<T: Into<Value>> From<T> for ValueOrRef {
	fn from(value: T) -> Self { Self::Value(value.into()) }
}

impl From<FieldRef> for ValueOrRef {
	fn from(field_ref: FieldRef) -> Self { Self::Ref(field_ref) }
}

#[derive(Debug, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenSchema {
	inner: TokenSchemaInner,
}

impl TokenSchema {
	pub fn new(schema: Schema) -> Self {
		Self {
			inner: TokenSchemaInner::Schema(schema),
		}
	}
	pub fn of<T: bevy::reflect::TypePath>() -> Self {
		Self {
			inner: TokenSchemaInner::Path(SmolStr::new_static(T::type_path())),
		}
	}
}

// sealed to protect Path variant from typepath/typename confusion
#[derive(Debug, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum TokenSchemaInner {
	/// The instance represents a dynamic type built at runtime
	Schema(Schema),
	/// The stable bevy [`TypePath::type_path`] to the type
	/// of this instance.
	/// This is not the [`std::any::type_name`], which
	/// is unstable.
	Path(SmolStr),
}

impl std::fmt::Display for TokenSchemaInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Path(s) => write!(f, "Path({})", s),
			Self::Schema(_) => write!(f, "Schema(..)"),
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
			match &token.value {
				ValueOrRef::Value(value) => doc.insert(&path, value)?,
				ValueOrRef::Ref(field_ref) => document_query
					.with_field(entity, field_ref, |value| {
						doc.insert(&path, &value)
					})
					.flatten()?,
			};
		}
		doc.xok()
	}
}



#[macro_export]
macro_rules! token2 {
    // Arm for: token2!(Name, Schema, default_value)
    (
        $(#[$meta:meta])*
        $new_ty:ident,
        $schema_ty:ident,
        $default_val:expr
    ) => {
        $(#[$meta])*
        #[derive(Reflect)]
        pub struct $new_ty(Token2);

        impl Default for $new_ty {
            fn default() -> Self {
                Self(
                    Token2::new_value::<$schema_ty>($default_val)
                        .expect("Failed to create Token2 with default value")
                )
            }
        }

        impl AsRef<Token2> for $new_ty {
            fn as_ref(&self) -> &Token2 {
                &self.0
            }
        }
    };

    // uses Token2::field
    // `token2!(Name, Schema)`
    (
        $(#[$meta:meta])*
        $new_ty:ident,
        $schema_ty:ident
    ) => {
        $(#[$meta])*
        #[derive(Reflect)]
        pub struct $new_ty(Token2);

        impl Default for $new_ty {
            fn default() -> Self {
                Self(Token2::new_field::<Self, $schema_ty>())
            }
        }

        impl AsRef<Token2> for $new_ty {
            fn as_ref(&self) -> &Token2 {
                &self.0
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
	}

	token2!(
			/// Some cool type
			/// This now works perfectly!
			#[derive(Debug, Clone)]
			Foo,
			Color
	);
	token2!(Bar, Color);

	token2!(Boo, Color, palettes::basic::GREEN.into());
}
