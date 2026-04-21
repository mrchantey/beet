use beet_core::prelude::*;



#[derive(SystemParam)]
pub struct StyleQuery<
	'w,
	's,
	D: 'static + QueryData = DefaultStyleData,
	F: 'static + QueryFilter = (),
> {
	commands: Commands<'w, 's>,
	query: Query<'w, 's, D, F>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
}


type DefaultStyleData = ();


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_name() {
		let mut world = World::new();
		world.with_state::<StyleQuery<(&Name, (&Name, &Name))>, _>(|_| {});
	}
}
