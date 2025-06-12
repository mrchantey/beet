use super::*;
use beet_common::prelude::*;
use bevy::prelude::*;



#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct HtmlDocument {
	pub head: Entity,
	pub body: Entity,
}


impl HtmlDocument {
	pub fn from_entity(mut entity: EntityWorldMut) -> Self {
		let is_element = entity.contains::<ElementNode>();
		let is_fragment = entity.contains::<FragmentNode>();
		if !is_element && !is_fragment {
			let head = entity.world_scope(|world| {
				world
					.spawn((
						NodeTag::new("head"),
						ElementNode::non_self_closing(),
						HtmlFragment::default(),
					))
					.id()
			});
			let body = entity.world_scope(|world| {
				world
					.spawn((
						NodeTag::new("body"),
						ElementNode::non_self_closing(),
						HtmlFragment::default(),
					))
					.id()
			});
			entity.insert(ChildOf(body));
			Self { head, body }
		} else {
			todo!()
		}
	}

	pub fn parse_bundle(bundle: impl Bundle) -> String {
		SharedTemplateApp::with(|app| {
			let entity = app.world_mut().spawn(bundle);
			let doc = Self::from_entity(entity);
			app.update();
			doc.collect_fragments(app.world_mut())
		})
	}

	fn collect_fragments(&self, world: &mut World) -> String {
		let head = world
			.entity_mut(self.head)
			.take::<HtmlFragment>()
			.expect("HtmlDocument Head has no HtmlFragment")
			.0;
		let body = world
			.entity_mut(self.body)
			.take::<HtmlFragment>()
			.expect("HtmlDocument Body has no HtmlFragment")
			.0;

		format!("<!DOCTYPE html><html>{head}{body}</html>")
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;
	use bevy::prelude::*;

	#[test]
	fn text() {
		HtmlDocument::parse_bundle(rsx! {hello world})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body>hello world</body></html>",
			);
	}
	#[test]
	fn elements() {
		HtmlDocument::parse_bundle(rsx! {<br/>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body><br/></body></html>",
			);
	}
	#[test]
	fn fragment() {
		HtmlDocument::parse_bundle(rsx! {<br/><br/>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body><br/><br/></body></html>",
			);
	}
	#[test]
	fn fragment_with_head() {
		HtmlDocument::parse_bundle(rsx! {<head><br/></head><br/>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head><br/></head><body><br/></body></html>",
			);
	}
	#[test]
	fn fragment_with_head_and_body() {
		HtmlDocument::parse_bundle(rsx! {<body><br/></body><!doctype pizza><head>7</head>})
			.xpect()
			.to_be(
				"<!doctype pizza><html><head>7</head><body><br/></body></html>",
			);
	}
	#[test]
	fn html_element() {
		HtmlDocument::parse_bundle(rsx! {<html><br/></html>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body><br/></body></html>",
			);
	}
}
