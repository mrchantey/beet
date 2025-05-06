use crate::prelude::*;

/// Collects all components with a `client:load` directive.
#[derive(Default)]
pub struct CollectClientIslands;

impl<T: AsRef<WebNode>> Pipeline<T, Vec<ClientIsland>>
	for CollectClientIslands
{
	fn apply(self, root: T) -> Vec<ClientIsland> {
		let mut islands = Vec::new();

		VisitRsxComponent::walk(
			root.as_ref(),
			|RsxComponent {
			     ron,
			     type_name,
			     tracker,
			     ..
			 }| {
				if let Some(ron) = ron {
					islands.push(ClientIsland {
						tracker: tracker.clone(),
						type_name: type_name.clone(),
						ron: ron.clone(),
					});
				}
			},
		);


		islands
	}
}

/// Representation of a component in an Rsx tree that was marked as an
/// island by a `client`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClientIsland {
	/// The [`RsxComponent::tracker`], will be used as the starting point
	/// with `register_effects`
	pub tracker: RustyTracker,
	/// The name of the component, retrieved via [`std::any::type_name`].
	pub type_name: String,
	/// The serialized component.
	pub ron: String,
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use serde::Deserialize;
	use serde::Serialize;
	#[allow(unused)]
	use sweet::prelude::*;

	#[allow(unused)]
	#[derive(Debug, PartialEq, Serialize, Deserialize, Node)]
	struct MyComponent {
		val: usize,
	}
	fn my_component(props: MyComponent) -> WebNode {
		rsx! { <div>{props.val}</div> }
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	fn collect_islands() {
		expect(
			rsx! { <MyComponent val=32 /> }
				.xpipe(CollectClientIslands::default())
				.len(),
		)
		.to_be(0);

		let island = &rsx! { <MyComponent client:load val=32 /> }
			.xpipe(CollectClientIslands::default())[0];

		expect(&island.type_name)
			.to_be("beet_rsx::rsx::pipeline::client_island::test::MyComponent");
		expect(&island.tracker)
			.to_be(&RustyTracker::new(0, 17560417869480573103));
		expect(&island.ron).to_be("(val:32)");
		expect(ron::de::from_str::<MyComponent>(&island.ron).unwrap())
			.to_be(MyComponent { val: 32 });
	}
}
