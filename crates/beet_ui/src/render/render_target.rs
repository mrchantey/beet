use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A format a tree renders to: a [`NodeRenderer`] that names the media types
/// it answers, registered in [`RenderTargets`].
///
/// The registry clones the registered instance for every render, so its
/// configuration (an [`HtmlRenderer`]'s indent, an [`AnsiTermRenderer`]'s
/// prefix) is set once at registration and its buffers start empty each time.
/// A target is handed exactly one of its [`media_types`](Self::media_types),
/// the negotiated one, as the only entry of [`RenderContext::accepts`], so one
/// serializing more than one format (the [`TemplateRenderer`]) knows which was
/// chosen.
pub trait RenderTarget: 'static + Send + Sync + Clone + NodeRenderer {
	/// The media types this target answers, preferred first.
	fn media_types(&self) -> Vec<MediaType>;
}

/// Every [`RenderTarget`] this world renders to, the one place a format is
/// chosen and the one path a render takes: [`render`](Self::render)
/// negotiates from the request's `Accept`, so a caller wanting a particular
/// type renders a request accepting it and gets exactly the body a client
/// asking for that type does.
///
/// Registered through [`AppRenderTargetExt::register_render_target`], the
/// built-in targets by [`RenderPlugin`] through the same call a downstream
/// target uses. For an overlapping media type the target registered last
/// wins, so a downstream crate replaces a built-in by registering its own.
#[derive(Default, Clone, Resource)]
pub struct RenderTargets {
	targets: Vec<Arc<dyn ErasedRenderTarget>>,
}

impl RenderTargets {
	/// The media type answering a request with no `Accept` or a wildcard: the
	/// web document, which a bare `curl` and a browser alike expect.
	pub const DEFAULT: MediaType = MediaType::Html;

	/// Register `target`, answering its media types ahead of any target
	/// registered before it.
	pub fn register(&mut self, target: impl RenderTarget) -> &mut Self {
		self.targets.push(Arc::new(target));
		self
	}

	/// Every media type a registered target answers, in registration order.
	pub fn available(&self) -> Vec<MediaType> {
		let mut available = Vec::new();
		for media_type in
			self.targets.iter().flat_map(|target| target.media_types())
		{
			if !available.contains(&media_type) {
				available.push(media_type);
			}
		}
		available
	}

	/// The media type `request` is answered as, [`DEFAULT`](Self::DEFAULT)
	/// for an absent `Accept` or a wildcard.
	///
	/// Each accepted type in order, a wildcard (`*/*`, `text/*`, ie a bare
	/// `curl` or an API Gateway default) read as the default, is answered by
	/// the first one a target is registered for. Failing that, any text type,
	/// or a wildcard, falls back to plain text, since prose reads in every
	/// text format; otherwise the mismatch names what was asked for and what
	/// is available.
	pub fn negotiate(
		&self,
		request: &RequestParts,
	) -> Result<MediaType, RenderError> {
		self.choose(request).map(|(media_type, _)| media_type)
	}

	/// Render the tree at `entity` as the answer to `request`, through the
	/// target its `Accept` negotiates to (see [`negotiate`](Self::negotiate)).
	///
	/// Settles the tree first: a one-shot render (a request, a publish step)
	/// happens between frames, so the `@` bindings built with the tree have not
	/// synced and a rule registered after the page's `<Stylesheet/>` baked has
	/// not reached it.
	pub fn render(
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
	) -> Result<MediaBytes, RenderError> {
		let (media_type, target) = world
			.get_resource::<Self>()
			.ok_or_else(|| {
				RenderError::Other(bevyhow!(
					"no `RenderTargets` in this world: add the `RenderPlugin`"
				))
			})?
			.choose(request)?;
		DocumentSync::settle(world);
		#[cfg(feature = "template")]
		crate::widgets::settle_stylesheets(world);
		target.render_as(world, entity, request, media_type)
	}

	/// The negotiated media type of [`negotiate`](Self::negotiate) and the
	/// target answering it.
	fn choose(
		&self,
		request: &RequestParts,
	) -> Result<(MediaType, Arc<dyn ErasedRenderTarget>), RenderError> {
		let accepts = request.accept();
		let candidates: Vec<MediaType> = match accepts.is_empty() {
			true => vec![Self::DEFAULT],
			false => accepts
				.iter()
				.map(|media_type| match media_type.is_wildcard() {
					true => Self::DEFAULT,
					false => media_type.clone(),
				})
				.collect(),
		};
		if let Some(chosen) = candidates.iter().find_map(|media_type| {
			self.target(media_type)
				.map(|target| (media_type.clone(), target))
		}) {
			return Ok(chosen);
		}
		match self.target(&MediaType::Text).filter(|_| {
			accepts.is_empty()
				|| accepts.iter().any(|media_type| {
					media_type.is_wildcard() || media_type.is_text()
				})
		}) {
			Some(target) => Ok((MediaType::Text, target)),
			None => Err(RenderError::AcceptMismatch {
				requested: candidates,
				available: self.available(),
			}),
		}
	}

	/// The target answering `media_type`, the last registered winning.
	fn target(
		&self,
		media_type: &MediaType,
	) -> Option<Arc<dyn ErasedRenderTarget>> {
		self.targets
			.iter()
			.rev()
			.find(|target| target.media_types().contains(media_type))
			.cloned()
	}
}

/// Registers a [`RenderTarget`] on an [`App`].
#[extend::ext(name=AppRenderTargetExt)]
pub impl App {
	/// Register `target` in the [`RenderTargets`], answering its media types
	/// ahead of any target registered before it.
	fn register_render_target(
		&mut self,
		target: impl RenderTarget,
	) -> &mut Self {
		self.world_mut()
			.get_resource_or_init::<RenderTargets>()
			.register(target);
		self
	}
}

/// Registers the built-in render targets: html, xml, markdown, plain text,
/// and with their features ansi (`style`) and the serialized scene
/// (`template_serde`).
#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
	fn build(&self, app: &mut App) {
		app.register_render_target(PlainTextRenderer::default())
			.register_render_target(HtmlRenderer::default())
			.register_render_target(XmlRenderer)
			.register_render_target(MarkdownRenderer::default());
		#[cfg(feature = "style")]
		app.register_render_target(AnsiTermRenderer::default());
		#[cfg(feature = "template_serde")]
		app.register_render_target(TemplateRenderer::default());
	}
}

/// The object-safe face of a [`RenderTarget`], which a registry can hold.
trait ErasedRenderTarget: 'static + Send + Sync {
	fn media_types(&self) -> Vec<MediaType>;
	fn render_as(
		&self,
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
		media_type: MediaType,
	) -> Result<MediaBytes, RenderError>;
}

impl<T: RenderTarget> ErasedRenderTarget for T {
	fn media_types(&self) -> Vec<MediaType> { RenderTarget::media_types(self) }

	fn render_as(
		&self,
		world: &mut World,
		entity: Entity,
		request: &RequestParts,
		media_type: MediaType,
	) -> Result<MediaBytes, RenderError> {
		NodeRenderer::render(
			&mut self.clone(),
			&mut RenderContext::new(world, entity, request)
				.with_negotiated(media_type),
		)
	}
}

impl RenderTarget for PlainTextRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }
}

impl RenderTarget for HtmlRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Html] }
}

impl RenderTarget for XmlRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Xml] }
}

impl RenderTarget for MarkdownRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Markdown] }
}

#[cfg(feature = "style")]
impl RenderTarget for AnsiTermRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::AnsiTerm] }
}

#[cfg(feature = "template_serde")]
impl RenderTarget for TemplateRenderer {
	fn media_types(&self) -> Vec<MediaType> { Self::available() }
}

#[cfg(test)]
mod test {
	#[allow(unused)]
	use crate::prelude::*;
	#[allow(unused)]
	use beet_core::prelude::*;
	#[allow(unused)]
	use beet_net::prelude::*;

	/// A world holding `<div><p>hi</p></div>`, parsed from html, with the
	/// built-in targets, and its root.
	#[cfg(feature = "bsx")]
	fn page() -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_html("<div><p>hi</p></div>");
		BsxParser::html()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		(world, entity)
	}

	/// The media type a request with `accept` (raw, `None` for no header)
	/// negotiates to, and the body.
	#[cfg(feature = "bsx")]
	fn negotiate(
		accept: Option<&str>,
	) -> Result<(MediaType, String), RenderError> {
		let (mut world, entity) = page();
		let mut request = RequestParts::default();
		if let Some(accept) = accept {
			request.headers.set_raw("accept", accept);
		}
		RenderTargets::render(&mut world, entity, &request)
			.map(|bytes| (bytes.media_type().clone(), bytes.to_string()))
	}

	/// No `Accept` renders the default media type.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn no_accept_renders_the_default() {
		negotiate(None)
			.unwrap()
			.xpect_eq((MediaType::Html, "<div><p>hi</p></div>".to_string()));
	}

	/// `accepts` is consulted in priority order, the first registered type
	/// winning, plain text an ordinary target like any other.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn accepts_in_priority_order() {
		negotiate(Some("text/html, text/plain"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
		negotiate(Some("text/markdown, text/html"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Markdown);
		negotiate(Some("text/plain"))
			.unwrap()
			.xpect_eq((MediaType::Text, "hi\n".to_string()));
		negotiate(Some("text/plain, text/html"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
	}

	/// A wildcard (`*/*`, `text/*`, eg a bare `curl` or an API Gateway
	/// default) resolves to the default media type, not the plain text
	/// fallback.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn wildcard_resolves_to_the_default() {
		negotiate(Some("*/*")).unwrap().0.xpect_eq(MediaType::Html);
		negotiate(Some("text/*"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
		negotiate(Some("image/png, */*"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
	}

	/// A text type with no target of its own falls back to plain text, even
	/// behind a type nothing renders.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn text_falls_back_to_plain_text() {
		negotiate(Some("text/css"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
		negotiate(Some("image/png, text/csv"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
	}

	/// Nothing renderable is a mismatch naming what was asked for and what is
	/// available.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn no_match_is_a_mismatch() {
		match negotiate(Some("image/png")) {
			Err(RenderError::AcceptMismatch {
				requested,
				available,
			}) => {
				requested.xpect_eq(vec![MediaType::Png]);
				for media_type in
					[MediaType::Html, MediaType::Markdown, MediaType::Text]
				{
					available.contains(&media_type).xpect_true();
				}
			}
			other => panic!("expected AcceptMismatch, got {other:?}"),
		}
	}

	/// The scene renderer answers every format it serializes, the selected
	/// one reaching it.
	#[cfg(all(feature = "bsx", feature = "template_serde", feature = "json"))]
	#[beet_core::test]
	fn serializes_the_scene() {
		negotiate(Some("application/json"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Json);
		#[cfg(feature = "postcard")]
		negotiate(Some("application/x-postcard"))
			.unwrap()
			.0
			.xpect_eq(MediaType::Postcard);
	}

	/// The terminal target answers ansi.
	#[cfg(all(feature = "bsx", feature = "style"))]
	#[beet_core::test]
	fn renders_ansi() {
		negotiate(Some("text/ansi-term"))
			.unwrap()
			.0
			.xpect_eq(MediaType::AnsiTerm);
	}

	/// A target registered from outside this crate's built-ins answers its own
	/// media type through the same call, one registered later replaces a
	/// built-in for the type both answer, and the negotiated type reaches it
	/// though the request's `Accept` names only a wildcard.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn registers_a_downstream_target() {
		#[derive(Clone)]
		struct Shout;
		impl NodeRenderer for Shout {
			fn render(
				&mut self,
				cx: &mut RenderContext,
			) -> Result<MediaBytes, RenderError> {
				let mut text = PlainTextRenderer::default();
				cx.walk(&mut text);
				MediaBytes::new_string(
					cx.accepts()[0].clone(),
					text.into_string().to_uppercase(),
				)
				.xok()
			}
		}
		impl RenderTarget for Shout {
			fn media_types(&self) -> Vec<MediaType> {
				vec![MediaType::other("text/x-shout"), MediaType::Html]
			}
		}
		let (mut world, entity) = page();
		world.resource_mut::<RenderTargets>().register(Shout);
		let accepting =
			|media_type| RequestParts::default().with_accept(media_type);
		RenderTargets::render(
			&mut world,
			entity,
			&accepting(MediaType::other("text/x-shout")),
		)
		.unwrap()
		.to_string()
		.xpect_contains("HI");
		let html = RenderTargets::render(
			&mut world,
			entity,
			&accepting(MediaType::Html),
		)
		.unwrap();
		html.to_string().xpect_contains("HI");
		html.media_type().clone().xpect_eq(MediaType::Html);
		RenderTargets::render(
			&mut world,
			entity,
			&accepting(MediaType::from_content_type("*/*")),
		)
		.unwrap()
		.media_type()
		.clone()
		.xpect_eq(MediaType::Html);
	}

	/// Parse markdown then render back as markdown.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn render_markdown() {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown("# Title");
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(entity), &bytes))
			.unwrap();
		RenderTargets::render(
			&mut world,
			entity,
			&RequestParts::default().with_accept(MediaType::Markdown),
		)
		.unwrap()
		.to_string()
		.trim()
		.xpect_eq("# Title");
	}
}
