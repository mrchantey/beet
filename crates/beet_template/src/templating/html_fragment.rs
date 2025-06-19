use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Add this component to have it populated with HTML on the next update cycle.
/// See [`HtmlDocument`] for hoisting and correct html structure.
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct HtmlFragment(pub String);

impl HtmlFragment {
	/// returns the HTML string representation of a given [`Bundle`].
	/// There are several transformations involved in the process,
	/// for example resolving slots, so we reuse a [`TemplateApp`]
	/// and run a full update cycle.
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		ReactiveApp::with(|app| {
			let entity = app
				.world_mut()
				.spawn((bundle, HtmlFragment::default()))
				.id();
			app.update();
			let value = app
				.world_mut()
				.entity_mut(entity)
				.take::<HtmlFragment>()
				.unwrap()
				.0;
			app.world_mut().despawn(entity);
			value
		})
	}
}

pub(super) fn render_html_fragments(
	mut query: Populated<(Entity, &mut HtmlFragment), Added<HtmlFragment>>,
	builder: Builder,
) {
	for (entity, mut html) in query.iter_mut() {
		builder.parse(entity, &mut html);
	}
}

/// Assign a javascript function that will collect events until hydration
pub(super) fn insert_event_playback_attribute(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	query: Populated<(Entity, &TreeIdx, &EventKey), Added<TreeIdx>>,
) {
	for (entity, idx, event) in query.iter() {
		let event_name = event.event_name();
		let js_func =
			format!("{}({}, event)", html_constants.event_handler, idx.inner());
		commands.spawn((
			AttributeOf::new(entity),
			AttributeKey(event_name.clone()),
			AttributeLit::new(js_func),
		));
	}
}


// TODO bench this approach vs concatenating parallel systems
#[rustfmt::skip]
#[derive(SystemParam)]
pub(super) struct Builder<'w, 's> {
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
		&'static AttributeKey,
		Option<&'static AttributeLit>
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
			// add attributes
			if let Some(attrs) = attributes {
				for (key, value) in attrs
					.iter()
					.filter_map(|attr| self.attributes.get(attr).ok())
				{
					html.push(' ');
					html.push_str(&key);
					if let Some(value) = &value {
						html.push_str("=\"");
						html.push_str(&value.to_string());
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

	#[test]
	fn works() {
		// doctype
		rsx! {<!doctype/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!DOCTYPE html>");
		// comment (in rstml must be quoted)
		rsx! {<!-- "howdy" -->}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!-- howdy -->");
		// raw text
		rsx! {howdy}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// quoted text
		rsx! {"howdy"}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// fragment
		rsx! {<>"howdy"</>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// block
		rsx! {{"howdy"}}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// self closing
		rsx! {<br/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<br/>");
		// not self closing
		rsx! {<span>hello</span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span>hello</span>");
		// child elements
		rsx! {<span><span>hello</span></span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span><span>hello</span></span>");
		// simple attribute
		rsx! {<div class="container"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\"></div>");
		// multiple attributes
		rsx! {<div class="container" id="main"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\" id=\"main\"></div>");
		// boolean attribute
		rsx! {<input disabled/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input disabled/>");
		// attribute in self-closing
		rsx! {<img src="image.jpg"/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<img src=\"image.jpg\"/>");
		// complex nested with attributes
		rsx! {<div class="wrapper"><span id="text">content</span></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be(
				"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
			);
		// expr attributes
		let val = true;
		rsx! {<input hidden=val/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input hidden=\"true\"/>");
	}

	#[test]
	fn templates() {
		#[template]
		fn Template() -> impl Bundle {
			rsx! {<div class="container"><span>hello</span></div>}
		}
		rsx! {
			"outer"
			<Template/>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("outer<div class=\"container\"><span>hello</span></div>");
	}
	#[test]
	fn events() {
		rsx! {<div onclick={||{}}/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be(
				"<div data-beet-rsx-idx=\"0\" onclick=\"_beet_event_handler(0, event)\"/>",
			);
	}
	#[test]
	fn signal_text_nodes() {
		let (get, _set) = signal("foo");
		rsx! {<div>{get}</div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div data-beet-rsx-idx=\"0\">foo</div>");
	}
}
