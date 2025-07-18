use crate::prelude::*;
use beet_core::prelude::*;
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
	/// The app disables loading snippets.
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		// TODO bench caching and reusing the app
		let mut app = App::new();
		app.add_plugins(TemplatePlugin)
			.insert_resource(TemplateFlags::None);
		let entity = app.world_mut().spawn(bundle).id();
		app.update();
		let html = app
			.world_mut()
			.run_system_cached_with(render_fragment, entity)
			.unwrap();
		app.world_mut().despawn(entity);
		html
	}
}
/// A parallelizable system to render all HTML fragments in the world.
pub(super) fn render_html_fragments(
	mut query: Populated<(Entity, &mut HtmlFragment), Added<HtmlFragment>>,
	builder: HtmlBuilder,
) {
	for (entity, mut html) in query.iter_mut() {
		builder.parse(entity, &mut html);
	}
}
/// A one-off system to render a single HTML fragment.
pub fn render_fragment(In(entity): In<Entity>, builder: HtmlBuilder) -> String {
	let mut str = String::new();
	builder.parse(entity, &mut str);
	str
}

/// Assign a javascript function that will collect events until hydration
pub(super) fn insert_event_playback_attribute(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	query: Populated<
		(&DomIdx, &Attributes),
		(With<EventTarget>, Added<DomIdx>),
	>,
	// potential event attributes
	attributes: Query<(Entity, &AttributeKey), Without<AttributeLit>>,
) {
	for (idx, attrs) in query.iter() {
		for (attr_entity, _) in attrs
			.iter()
			.filter_map(|attr| attributes.get(attr).ok())
			.filter(|(_, key)| key.starts_with("on"))
		{
			commands
				.entity(attr_entity)
				.insert(AttributeLit::new(format!(
					"{}({}, event)",
					html_constants.event_handler,
					idx.inner()
				)));
		}
	}
}


// TODO bench this approach vs concatenating parallel systems
#[rustfmt::skip]
#[derive(SystemParam)]
pub struct HtmlBuilder<'w, 's> {
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

impl HtmlBuilder<'_, '_> {
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
			if element.self_closing && **tag == "style" {
				panic!("self closing style tags are not allowed in HTML");
			}

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
	fn doctype() {
		rsx! {<!doctype/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!DOCTYPE html>");
	}

	#[test]
	fn comment() {
		// comment (in rstml must be quoted)
		rsx! {<!-- "howdy" -->}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!-- howdy -->");
	}

	#[test]
	fn raw_text() {
		rsx! {howdy}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
	}

	#[test]
	fn quoted_text() {
		rsx! {"howdy"}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
	}

	#[test]
	fn fragment() {
		rsx! {<>"howdy"</>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
	}

	#[test]
	fn block() {
		rsx! {{"howdy"}}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
	}

	#[test]
	fn self_closing_tag() {
		rsx! {<br/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<br/>");
	}

	#[test]
	fn not_self_closing_tag() {
		rsx! {<span>hello</span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span>hello</span>");
	}

	#[test]
	fn child_elements() {
		rsx! {<span><span>hello</span></span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span><span>hello</span></span>");
	}

	#[test]
	fn simple_attribute() {
		rsx! {<div class="container"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\"></div>");
	}

	#[test]
	fn multiple_attributes() {
		rsx! {<div class="container" id="main"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\" id=\"main\"></div>");
	}

	#[test]
	fn boolean_attribute() {
		rsx! {<input disabled/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input disabled/>");
	}

	#[test]
	fn attribute_in_self_closing_tag() {
		rsx! {<img src="image.jpg"/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<img src=\"image.jpg\"/>");
	}

	#[test]
	fn complex_nested_with_attributes() {
		rsx! {<div class="wrapper"><span id="text">content</span></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be(
				"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
			);
	}

	#[test]
	fn expr_attributes() {
		let val = true;
		rsx! {<input hidden=val/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input hidden=\"true\"/>");
	}

	#[template]
	fn Template() -> impl Bundle {
		rsx! {<div class="container"><span>hello</span></div>}
	}
	#[test]
	fn templates() {
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
			.xmap(HtmlDocument::parse_bundle)
			.xpect()
			.to_contain(
				"<div onclick=\"_beet_event_handler(0, event)\" data-beet-dom-idx=\"0\"/>",
			);
	}
	#[test]
	fn iterators() {
		rsx! {
			<div>{vec![
				rsx! {<span>foo</span>},
				rsx! {<span>bar</span>},
				rsx! {<span>baz</span>},
					]}
			</div>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<div><span>foo</span><span>bar</span><span>baz</span></div>");
	}
	#[test]
	fn signal_text_nodes() {
		let (get, _set) = signal("foo");
		rsx! {<div>{get}</div>}
			.xmap(HtmlDocument::parse_bundle)
			.xpect()
			.to_contain("<div data-beet-dom-idx=\"0\">foo</div>");
	}
}
