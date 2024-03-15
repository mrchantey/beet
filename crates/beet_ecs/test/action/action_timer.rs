// use bevy::prelude::*;
// use beet_ecs::common_actions::*;
// use beet_ecs::common_selectors::*;
// use beet_ecs::*;
// use sweet::*;

// #[sweet_test]
// pub fn works() -> Result<()> {
// 	let mut app = App::new();
// 	let my_tree = || {
// 		tree! {
// 			<sequence apply_deferred>
// 				<succeed_in_one_second apply_deferred/>
// 				<succeed_in_one_second apply_deferred/>
// 			</sequence>
// 		}
// 	};

// 	app.add_plugins(TreePlugin::new(my_tree));
// 	app.insert_time();

// 	let entity = app.world.spawn(TreeBundle::new(my_tree)).id();

// 	app.update_with_millis(1); //start running 1

// 	let out = PropTree::<ActionTimer>::new(my_tree, &app.world, entity);
// 	expect(out.children[0].value).to_be_some()?;
// 	let out = PropTree::<ActionResult>::new(my_tree, &app.world, entity);
// 	expect(out.children[0].value).to_be_none()?;
// 	expect(out.children[1].value).to_be_none()?;

// 	app.update_with_secs(1); //end running 1

// 	let out = PropTree::<ActionResult>::new(my_tree, &app.world, entity);
// 	expect(out.children[0].value).to_be_some()?;
// 	expect(out.children[1].value).to_be_none()?;
// 	let out = PropTree::<Running>::new(my_tree, &app.world, entity);
// 	expect(out.children[0].value).to_be_none()?;
// 	expect(out.children[1].value).to_be_none()?;

// 	app.update(); //start running 2
// 	app.update_with_secs(1); //end running 2

// 	let out = PropTree::<ActionResult>::new(my_tree, &app.world, entity);
// 	expect(out.children[0].value).to_be_none()?;
// 	expect(out.children[1].value).to_be_some()?;

// 	Ok(())
// }
