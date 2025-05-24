use beet_common::prelude::*;
use bevy::ecs::system::RunSystemError;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// returns the HTML string representation of a given [`Bundle`]
pub fn bundle_to_html_oneshot(
	bundle: impl Bundle,
) -> Result<String, RunSystemError> {
	let mut app = App::new();
	let entity = app.world_mut().spawn(bundle).id();
	app.world_mut().run_system_once_with(node_to_html, entity)
}


fn node_to_html(node: In<Entity>, builder: Builder) -> String {
	let mut html = String::new();
	builder.parse(*node, &mut html);
	html
}


// TODO bench this approach vs flat multi-threaded
#[rustfmt::skip]
#[derive(SystemParam)]
struct Builder<'w, 's> {
	elements: Query<'w,'s,(
		&'static ElementNode,
		&'static NodeTag,
		Option<&'static Attributes>,
		Option<&'static Children>
	)>,
	fragments: Query<'w, 's,(
		&'static FragmentNode,
		&'static Children
	)>,
	attributes: Query<'w,'s,(
		&'static AttributeKeyStr,
		Option<&'static AttributeValueStr>
	)>,
	doctypes: Query<'w, 's, &'static DoctypeNode>,
	comments: Query<'w, 's, &'static CommentNode>,
	texts: Query<'w, 's, &'static TextNode>,
}

impl Builder<'_, '_> {
	fn parse(&self, entity: Entity, html: &mut String) {
		if let Ok(_) = self.doctypes.get(entity) {
			html.push_str("<!DOCTYPE html>");
		}
		if let Ok(comment) = self.comments.get(entity) {
			html.push_str(&format!("<!-- {} -->", comment.0));
		}
		if let Ok(text) = self.texts.get(entity) {
			html.push_str(&text.0);
		}
		if let Ok((_, children)) = self.fragments.get(entity) {
			for child in children.iter() {
				self.parse(child, html);
			}
		}
		if let Ok((element, tag, attributes, children)) =
			self.elements.get(entity)
		{
			html.push_str(&format!("<{}", tag.0));
			if let Some(attrs) = attributes {
				for (key, value) in attrs
					.iter()
					.filter_map(|attr| self.attributes.get(attr).ok())
				{
					html.push(' ');
					html.push_str(&key);
					if let Some(value) = &value {
						html.push_str("=\"");
						html.push_str(value);
						html.push_str("\"");
					}
				}
			}
			if element.self_closing {
				html.push_str("/>");
				return;
			} else {
				html.push('>');
			}
			if let Some(children) = children {
				for child in children.iter() {
					self.parse(child, html);
				}
			}
			html.push_str(&format!("</{}>", tag.0));
		}
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	fn parse(bundle: impl Bundle) -> Matcher<String> {
		bundle.xmap(bundle_to_html_oneshot).unwrap().xpect()
	}

	#[test]
	fn works() {
		// doctype
		rsx! {<!doctype/>}.xmap(parse).to_be("<!DOCTYPE html>");
		// comment (in rstml must be quoted)
		rsx! {<!-- "howdy" -->}.xmap(parse).to_be("<!-- howdy -->");
		// raw text
		rsx! {howdy}.xmap(parse).to_be("howdy");
		// quoted text
		rsx! {"howdy"}.xmap(parse).to_be("howdy");
		// fragment
		rsx! {<>"howdy"</>}.xmap(parse).to_be("howdy");
		// block
		rsx! {{"howdy"}}.xmap(parse).to_be("howdy");
		// self closing
		rsx! {<br/>}.xmap(parse).to_be("<br/>");
		// not self closing
		rsx! {<span>hello</span>}
			.xmap(parse)
			.to_be("<span>hello</span>");
		// child elements
		rsx! {<span><span>hello</span></span>}
			.xmap(parse)
			.to_be("<span><span>hello</span></span>");
		// simple attribute
		rsx! {<div class="container"></div>}
			.xmap(parse)
			.to_be("<div class=\"container\"></div>");
		// multiple attributes
		rsx! {<div class="container" id="main"></div>}
			.xmap(parse)
			.to_be("<div class=\"container\" id=\"main\"></div>");
		// boolean attribute
		rsx! {<input disabled/>}
			.xmap(parse)
			.to_be("<input disabled/>");
		// attribute in self-closing
		rsx! {<img src="image.jpg"/>}
			.xmap(parse)
			.to_be("<img src=\"image.jpg\"/>");
		// complex nested with attributes
		rsx! {<div class="wrapper"><span id="text">content</span></div>}
			.xmap(parse)
			.to_be(
				"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
			);
		// expr attributes
		let val = true;
		rsx! {<input hidden=val/>}
			.xmap(parse)
			.to_be("<input hidden=\"true\"/>");
	}

	#[test]
	#[ignore = "wip"]
	fn templates() {
		#[template]
		fn Template() -> impl Bundle {
			rsx! {<div class="container"><span>hello</span></div>}
		}
		rsx! {
			"outer"
			<Template/>
		}
		.xmap(parse)
		.to_be("outer<div class=\"container\"><span>hello</span></div>");
	}
}
