use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

#[derive(Debug, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token2 {
	field_ref: FieldRef,
	schema: InstanceSchema,
}

impl Token2 {
	pub fn new(field_ref: FieldRef, schema: InstanceSchema) -> Self {
		Self { field_ref, schema }
	}
	pub fn of<T: TypePath, Schema: TypePath>() -> Self {
		Self {
			field_ref: FieldRef::of::<T>(),
			schema: InstanceSchema::of::<Schema>(),
		}
	}
	pub fn of_with_reflect<Token: TypePath, Val: Typed>(
		value: Val,
	) -> Result<Self> {
		let value = Value::from_reflect(&value)?;
		Self {
			field_ref: FieldRef::of::<Token>().with_init(value),
			schema: InstanceSchema::of::<Val>(),
		}
		.xok()
	}
	#[cfg(feature = "json")]
	pub fn of_with_serde<Token: TypePath, Val: Serialize + TypePath>(
		value: Val,
	) -> Result<Self> {
		let value = Value::from_serde(&value)?;
		Self {
			field_ref: FieldRef::of::<Token>().with_init(value),
			schema: InstanceSchema::of::<Val>(),
		}
		.xok()
	}
}



#[macro_export]
macro_rules! token2 {
    (
        $(#[$meta:meta])* // Captures doc comments and attributes
        $new_ty:ident,
        $schema_ty:ident
    ) => {
        $(#[$meta])* // Expands them onto the struct
        #[derive(Reflect)]
        pub struct $new_ty(Token2);

        impl Default for $new_ty {
            fn default() -> Self {
                Self(Token2::of::<Self, $schema_ty>())
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

	token2!(
			/// Some cool type
			/// This now works perfectly!
			#[derive(Debug, Clone)]
			Foo,
			Color
	);
	token2!(Bar, Color);
}
