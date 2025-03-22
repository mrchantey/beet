use crate::prelude::*;

/// Representation of a component in an Rsx tree that was marked as an
/// island by a `client`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClientIsland {
	/// The location of the component, will be used as the starting point
	/// with `register_effects`
	pub location: TreeLocation,
	/// The name of the component, retrieved via [`std::any::type_name`].
	pub type_name: String,
	/// The serialized component.
	pub ron: String,
}

impl ClientIsland {
	/// Convert the island into a token stream that can be used to mount the
	/// island.
	///
	/// ## Panics
	///
	/// Panics if the type name or ron string are not valid tokens.
	#[cfg(feature = "parser")]
	pub fn into_mount_tokens(&self) -> proc_macro2::TokenStream {
		let location = &self.location;
		let type_name =
			self.type_name.parse::<proc_macro2::TokenStream>().unwrap();
		let ron = &self.ron;
		quote::quote! {
			beet::exports::ron::de::from_str::<#type_name>(#ron)?
				.render()
				.pipe(RegisterEffects::new(#location))?;
		}
	}
}


/// Collects all components with a `client:load` directive.
#[derive(Default)]
pub struct CollectClientIslands;

impl RsxPipelineTarget for Vec<ClientIsland> {}

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



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	#[allow(unused)]
	#[derive(Debug, PartialEq, Serialize, Deserialize, Node)]
	struct MyComponent {
		val: usize,
	}
	fn my_component(props: MyComponent) -> RsxRoot {
		rsx! { <div>{props.val}</div> }
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	fn collect_islands() {
		expect(
			rsx! { <MyComponent val=32 /> }
				.pipe(CollectClientIslands::default())
				.len(),
		)
		.to_be(0);

		let island = &rsx! { <MyComponent client:load val=32 /> }
			.pipe(CollectClientIslands::default())[0];

		expect(&island.type_name)
			.to_be("beet_rsx::rsx::pipeline::client_island::test::MyComponent");
		expect(&island.location).to_be(&TreeLocation::new(0, 0, 0));
		expect(&island.ron).to_be("(val:32)");
		expect(ron::de::from_str::<MyComponent>(&island.ron).unwrap())
			.to_be(MyComponent { val: 32 });
	}

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		let island = ClientIsland {
			location: TreeLocation::new(1, 2, 3),
			type_name: "MyComponent".into(),
			ron: "(val:32)".into(),
		};
		let location = &island.location;

		expect(island.into_mount_tokens().to_string()).to_be(
			quote::quote! {
				beet::exports::ron::de::from_str::<MyComponent>((val:32))?
					.render()
					.pipe(RegisterEffects::new(#location))?;
			}
			.to_string(),
		);
	}
}
