#![allow(unused)]
use crate::prelude::*;





/// Collects all components with a `client:load` directive.
#[derive(Default)]
pub struct CollectClientIslands;


impl<T: RsxPipelineTarget + AsRef<RsxNode>> RsxPipeline<T, Vec<ClientIsland>>
	for CollectClientIslands
{
	fn apply(self, root: T) -> Vec<ClientIsland> {
		let mut islands = Vec::new();

		TreeLocationVisitor::visit(root.as_ref(), |loc, node| match node {
			RsxNode::Component(RsxComponent { ron, type_name, .. }) => {
				if let Some(ron) = ron {
					islands.push(ClientIsland {
						location: loc.clone(),
						type_name: type_name.clone(),
						ron: ron.clone(),
					});
				}
			}
			_ => {}
		});

		islands
	}
}


impl RsxPipelineTarget for Vec<ClientIsland> {}


/// Representation of a component in an Rsx tree that was marked as an
/// island by a `client`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientIsland {
	/// The location of the component, will be used as the starting point
	/// with `register_effects`
	location: TreeLocation,
	/// The name of the component, retrieved via [`std::any::type_name`].
	type_name: String,
	/// The serialized component.
	ron: String,
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;


	#[derive(Debug, PartialEq, Serialize, Deserialize, Node)]
	struct MyComponent {
		val: usize,
	}
	fn my_component(props: MyComponent) -> RsxRoot {
		rsx! { <div>{props.val}</div> }
	}

	#[test]
	fn works() {
		expect(
			rsx! { <MyComponent val=32 /> }
				.pipe(CollectClientIslands::default())
				.len(),
		)
		.to_be(0);

		let island = &rsx! { <MyComponent client:load val=32 /> }
			.pipe(CollectClientIslands::default())[0];

		expect(&island.type_name).to_be(
			"beet_rsx::rsx::pipeline::collect_client_islands::test::MyComponent",
		);
		expect(&island.location).to_be(&TreeLocation::new(0, 0, 0));
		expect(&island.ron).to_be("(val:32)");
		expect(ron::de::from_str::<MyComponent>(&island.ron).unwrap())
			.to_be(MyComponent { val: 32 });
	}
}
