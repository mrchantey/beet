//! The controls' side of the [`WritePolicy`]: holding a `Blur` control's
//! write while it has the focus, and releasing it the moment it loses it.
//!
//! The write-back (`sync_local_to_document`) knows nothing of focus; it
//! leaves a [`WriteHeld`] control alone. The focus model is what marks one:
//! focusing a [`WritePolicy::Blur`] control holds its write, and the blur
//! releases it *synchronously*, in the observer of the focus leaving, so on
//! every surface the write has landed before whatever took the focus acts.
//! On the terminal a click is `PointerDown` (focus moves) then `PointerUp`
//! (the button's handler); in the browser `focusout` is dispatched before
//! the `click` it precedes. Either way an action reading the document reads
//! the blurred edit.
//!
//! The two projection selects, whose local value is not their field's
//! ([`EntityPicker`], [`VariantSelect`]), release through their own write
//! rather than the field write-back, so a held pick lands as the reference or
//! variant it names.
use crate::prelude::*;
use crate::widgets::schema_ui::entity_picker::EntityPicker;
use crate::widgets::schema_ui::variant_select::VariantSelect;
use beet_core::prelude::*;

/// Observer: a [`WritePolicy::Blur`] control taking the focus holds its
/// write until the blur. Watches the policy too, since a generated control's
/// spread lands a frame after the element it is on.
pub(super) fn hold_write_on_focus(
	ev: On<Insert, (Focus, WritePolicy)>,
	controls: Query<&WritePolicy, With<Focus>>,
	mut commands: Commands,
) {
	if controls
		.get(ev.entity)
		.is_ok_and(|policy| *policy == WritePolicy::Blur)
	{
		commands.entity(ev.entity).insert(WriteHeld);
	}
}

/// Observer: a held control losing the focus releases its write, landing the
/// edit in its document at once. A despawn removes the focus too, so a
/// control taken away mid-edit lands what it held rather than losing it.
pub(super) fn release_write_on_blur(
	ev: On<Remove, Focus>,
	held: Query<
		(
			&Value,
			Option<&FieldRef>,
			Option<&SyncedValue>,
			Option<&EntityPicker>,
			Option<&VariantSelect>,
		),
		With<WriteHeld>,
	>,
	mut docs: DocumentQuery,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let Ok((value, field, synced, picker, variant)) = held.get(entity) else {
		return OK;
	};
	commands.entity(entity).try_remove::<WriteHeld>();
	match (picker, variant, field) {
		(Some(picker), _, _) => picker.write(entity, value, &mut docs),
		(_, Some(variant), _) => variant.write(entity, value, &mut docs),
		(_, _, Some(field)) => docs.write_back(entity, field, value, synced),
		_ => OK,
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::widgets::test_ext;
	use beet_core::prelude::*;

	/// A `Blur` text field over `count`, and an action button whose press
	/// records what the document held when it read it.
	fn build() -> (World, Entity, Entity, Store<Value>) {
		let mut world = test_ext::form_world();
		let seen = Store::new(Value::Null);
		let root = world
			.spawn_template(rsx! {
				<div>
					<NumberField field={FieldRef::new("count")} write={WritePolicy::Blur}/>
					<Button action=true>"Act"</Button>
				</div>
			})
			.unwrap()
			.id();
		world
			.entity_mut(root)
			.insert(Document::new(value!({ "count": 1 })));
		test_ext::settle_world(&mut world);
		let button = test_ext::element_in(&mut world, "button");
		let read = seen.clone();
		world.entity_mut(button).observe(
			move |ev: On<PointerUp>, mut docs: DocumentQuery| -> Result {
				let scene =
					docs.field_value(ev.event_target(), &FieldRef::default())?;
				read.set(scene);
				OK
			},
		);
		(world, root, button, seen)
	}

	fn document(world: &mut World, root: Entity) -> Value {
		world.entity(root).get::<Document>().unwrap().0.clone()
	}

	/// A focused `Blur` control's edit stays local, and the blur lands it.
	#[beet_core::test]
	fn a_held_edit_lands_on_blur() {
		let (mut world, root, _, _) = build();
		let input = test_ext::element_in(&mut world, "input");
		world.entity_mut(input).insert(Focus);
		*world.entity_mut(input).get_mut::<Value>().unwrap() = Value::Int(2);
		test_ext::settle_world(&mut world);
		document(&mut world, root).xpect_eq(value!({ "count": 1 }));

		world.entity_mut(input).remove::<Focus>();
		document(&mut world, root).xpect_eq(value!({ "count": 2 }));
		world.entity(input).contains::<WriteHeld>().xpect_false();
	}

	/// A held control despawned mid-edit (its generation replaced under it)
	/// lands what it held on the way out.
	#[beet_core::test]
	fn a_despawned_control_lands_its_held_edit() {
		let (mut world, root, _, _) = build();
		let input = test_ext::element_in(&mut world, "input");
		world.entity_mut(input).insert(Focus);
		*world.entity_mut(input).get_mut::<Value>().unwrap() = Value::Int(2);
		test_ext::settle_world(&mut world);
		world.entity_mut(input).despawn();
		test_ext::settle_world(&mut world);
		document(&mut world, root).xpect_eq(value!({ "count": 2 }));
	}

	/// The terminal's ordering: a click on a button is the press that moves
	/// the focus, then the release that runs the button, queued as the hit
	/// test queues them, so the handler reads the edit the press blurred.
	#[beet_core::test]
	fn a_click_lands_the_held_edit_before_the_handler_reads() {
		let (mut world, _, button, seen) = build();
		let input = test_ext::element_in(&mut world, "input");
		world.entity_mut(input).insert(Focus);
		*world.entity_mut(input).get_mut::<Value>().unwrap() = Value::Int(2);
		test_ext::settle_world(&mut world);

		let pointer = world.spawn_empty().id();
		world
			.commands()
			.entity(button)
			.trigger(PointerDown::new(pointer));
		world
			.commands()
			.entity(button)
			.trigger(PointerUp::new(pointer));
		world.flush();
		seen.get().xpect_eq(value!({ "count": 2 }));
		world.entity(button).contains::<Focus>().xpect_true();
		world.entity(input).contains::<Focus>().xpect_false();
	}
}
