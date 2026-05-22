//! Reactive document fields: a child [`Value`] mirrors a [`Document`] field
//! and stays in sync as the document changes.
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// Schema describing the document, enabling typed field writes.
#[derive(Reflect)]
#[allow(dead_code)]
struct CountDoc {
	count: i64,
}

fn main() -> Result {
	let mut world = DocumentPlugin::world();
	let doc = world
		.spawn((
			Document::new(val!({ "count": 7i64 })),
			DocumentSchema::of::<CountDoc>(),
			children![(Value::default(), FieldRef::new("count"))],
		))
		.id();

	// mirror the document field onto the child Value
	world.update_local();
	mirrored(&mut world).xpect_eq(Value::Int(7));

	// mutate the document; the change propagates on the next update
	{
		let mut entity = world.entity_mut(doc);
		let mut document = entity.get_mut::<Document>().unwrap();
		let count = document.get_field_mut(&[FieldSegment::key("count")])?;
		*count = Value::Int(count.as_i64().unwrap_or(0) + 1);
	}

	world.update_local();
	mirrored(&mut world).xpect_eq(Value::Int(8));

	println!("success");
	Ok(())
}

/// The current value mirrored onto the field's [`Value`] component.
fn mirrored(world: &mut World) -> Value {
	world.query_once::<(&Value, &FieldRef)>()[0].0.clone()
}
