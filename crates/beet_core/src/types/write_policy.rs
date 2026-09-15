//! [`WritePolicy`]: when a control's edit lands in the document it binds.
use crate::prelude::*;

/// When a bound control's local edit is written into its document field.
///
/// Every control edits its own [`Value`] freely; the policy decides when the
/// write-back carries that edit into the document, and so when everything
/// bound to the same field (a heading, a tree row, the store) sees it. It is
/// data: a [`NamedFieldSchema`] declares it for a field
/// ([`NamedFieldSchema::write`]), a kind supplies the default where none is
/// declared ([`ValueSchema::write_policy`]), and the generated control carries
/// the resolved policy as this component. A control carrying none writes as
/// [`Input`](Self::Input).
///
/// There is no draft and no apply: a policy is the whole of *when*, and a
/// refused write (a cycle, a schema commit that would strand existing rows) is
/// reverted by the layer that refused it, never held for a later button.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WritePolicy {
	/// Every change writes: a keystroke, a toggle, a pick. The default for
	/// text and booleans, where a partial edit is a legitimate value and the
	/// page following each key is the point.
	#[default]
	Input,
	/// Changes are held while the control is focused and written when it
	/// loses focus, as one edit. The default for parsed and referential
	/// fields: a number mid-edit is not yet the number, and a reference
	/// refined key by key (type-ahead on a select) would restructure the
	/// document once per refinement.
	Blur,
	/// The write-back never writes: an action consumes the control's value
	/// in one atomic write of its own (the add button reading the key typed
	/// beside it, a structural edit's press), or a form's submit gathers it.
	Action,
}

/// Marks a [`WritePolicy::Blur`] control between focus and blur: its edits are
/// held, and the write-back leaves it alone until the blur releases them.
///
/// Held by the control rather than read off the focus model, since the
/// write-back knows nothing of focus: whoever owns focus on a surface holds
/// the write on focus and releases it on blur.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct WriteHeld;

impl ValueSchema {
	/// The [`WritePolicy`] a control for this kind writes under when its field
	/// declares none.
	///
	/// Text and booleans write on [`Input`](WritePolicy::Input); numbers and
	/// entity references on [`Blur`](WritePolicy::Blur); a composite has no
	/// control of its own and is edited by [`Action`](WritePolicy::Action).
	/// An `Optional` answers as its inner schema; a `Ref` has no kind until
	/// resolved and answers as text, the leaf a form gives it once it is.
	pub fn write_policy(&self) -> WritePolicy {
		match self {
			Self::Bool(_) | Self::String(_) | Self::Enum(_) => {
				WritePolicy::Input
			}
			Self::I64(_) | Self::U64(_) | Self::F64(_) | Self::Entity(_) => {
				WritePolicy::Blur
			}
			Self::Struct(_)
			| Self::Tuple(_)
			| Self::List(_)
			| Self::Map(_)
			| Self::Bytes(_)
			| Self::Any
			| Self::Null => WritePolicy::Action,
			Self::Optional(inner) => inner.write_policy(),
			Self::Ref(_) => WritePolicy::Input,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// The per-kind defaults: text and choices land as typed or picked,
	/// parsed and referential fields on blur, structure by action.
	#[crate::test]
	fn kinds_default_by_what_a_partial_edit_means() {
		ValueSchema::String(default())
			.write_policy()
			.xpect_eq(WritePolicy::Input);
		ValueSchema::Bool(default())
			.write_policy()
			.xpect_eq(WritePolicy::Input);
		ValueSchema::I64(default())
			.write_policy()
			.xpect_eq(WritePolicy::Blur);
		ValueSchema::Entity(default())
			.write_policy()
			.xpect_eq(WritePolicy::Blur);
		ValueSchema::Optional(Box::new(ValueSchema::F64(default())))
			.write_policy()
			.xpect_eq(WritePolicy::Blur);
		ValueSchema::List(default())
			.write_policy()
			.xpect_eq(WritePolicy::Action);
	}
}
