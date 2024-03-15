use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();

	let child1 = app.world.spawn(Score::Fail).id();

	let _parent = app.world.spawn(Edges::new().with_child(child1));
	app.add_systems(Update, changes_score_to_pass);

	expect(&app)
		.component::<Score>(child1)?
		.to_be(&Score::Fail)?;
	app.update();
	expect(&app)
		.component::<Score>(child1)?
		.to_be(&Score::Pass)?;

	Ok(())
}


fn changes_score_to_pass(
	parents: Query<&Edges>,
	mut children: Query<&mut Score>,
) {
	for edges in parents.iter() {
		for child in edges.iter() {
			if let Ok(mut score) = children.get_mut(*child) {
				*score = Score::Pass;
			}
		}
	}
}
