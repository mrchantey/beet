use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// A format a tree renders to: a [`NodeRenderer`] that names the media types
/// it answers, registered in [`RenderTargets`].
///
/// The registry clones the registered instance for every render, so its
/// configuration (an [`HtmlRenderer`]'s indent, an [`AnsiTermRenderer`]'s
/// prefix) is set once at registration and its buffers start empty each time.
/// A target is handed exactly one of its [`media_types`](Self::media_types)
/// as the only entry of [`RenderContext::accepts`], so one serializing more
/// than one format (the [`TemplateRenderer`]) knows which was chosen.
pub trait RenderTarget: 'static + Send + Sync + Clone + NodeRenderer {
	/// The media types this target answers, preferred first.
	fn media_types(&self) -> Vec<MediaType>;
}

/// Every [`RenderTarget`] this world renders to, the one place a format is
/// chosen and the one path a render takes, whether a request negotiated it
/// from `Accept` or a caller named it.
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

	/// The media type a request accepting `accepts` is answered as, `default`
	/// being the answer to an empty list or a wildcard.
	///
	/// Each accepted type in order, a wildcard (`*/*`, `text/*`, ie a bare
	/// `curl` or an API Gateway default) read as `default`, is answered by the
	/// first one a target is registered for. Failing that, any text type, or a
	/// wildcard, falls back to plain text, since prose reads in every text
	/// format; otherwise the mismatch names what was asked for and what is
	/// available.
	pub fn negotiate(
		&self,
		accepts: &[MediaType],
		default: &MediaType,
	) -> Result<MediaType, RenderError> {
		let candidates: Vec<MediaType> = match accepts.is_empty() {
			true => vec![default.clone()],
			false => accepts
				.iter()
				.map(|media_type| match media_type.is_wildcard() {
					true => default.clone(),
					false => media_type.clone(),
				})
				.collect(),
		};
		if let Some(media_type) = candidates
			.iter()
			.find(|media_type| self.target(media_type).is_some())
		{
			return Ok(media_type.clone());
		}
		let falls_back = self.target(&MediaType::Text).is_some()
			&& (accepts.is_empty()
				|| accepts.iter().any(|media_type| {
					media_type.is_wildcard() || media_type.is_text()
				}));
		match falls_back {
			true => Ok(MediaType::Text),
			false => Err(RenderError::AcceptMismatch {
				requested: candidates,
				available: self.available(),
			}),
		}
	}

	/// Render the tree at `entity` as `media_type`, through the target
	/// registered for it.
	///
	/// Settles the tree first: a one-shot render (a request, a publish step)
	/// happens between frames, so the `@` bindings built with the tree have not
	/// synced and a rule registered after the page's `<Stylesheet/>` baked has
	/// not reached it.
	pub fn render(
		world: &mut World,
		entity: Entity,
		media_type: &MediaType,
	) -> Result<MediaBytes, RenderError> {
		let target = world
			.get_resource::<Self>()
			.and_then(|targets| targets.target(media_type))
			.ok_or_else(|| RenderError::AcceptMismatch {
				requested: vec![media_type.clone()],
				available: world
					.get_resource::<Self>()
					.map(Self::available)
					.unwrap_or_default(),
			})?;
		DocumentSync::settle(world);
		#[cfg(feature = "template")]
		crate::widgets::settle_stylesheets(world);
		target.render_as(entity, world, media_type)
	}

	/// Render the tree at `entity` as the media type `accepts` negotiates to
	/// (see [`negotiate`](Self::negotiate)), the render a request makes.
	pub fn render_negotiated(
		world: &mut World,
		entity: Entity,
		accepts: &[MediaType],
		default: &MediaType,
	) -> Result<MediaBytes, RenderError> {
		let media_type = world
			.get_resource::<Self>()
			.ok_or_else(|| {
				RenderError::Other(bevyhow!(
					"no `RenderTargets` in this world: add the `RenderPlugin`"
				))
			})?
			.negotiate(accepts, default)?;
		Self::render(world, entity, &media_type)
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

/// Registers the built-in render targets: html, markdown, plain text, and
/// with their features ansi (`style`) and the serialized scene
/// (`template_serde`).
#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
	fn build(&self, app: &mut App) {
		app.register_render_target(PlainTextRenderer::default())
			.register_render_target(HtmlRenderer::default())
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
		entity: Entity,
		world: &mut World,
		media_type: &MediaType,
	) -> Result<MediaBytes, RenderError>;
}

impl<T: RenderTarget> ErasedRenderTarget for T {
	fn media_types(&self) -> Vec<MediaType> { RenderTarget::media_types(self) }

	fn render_as(
		&self,
		entity: Entity,
		world: &mut World,
		media_type: &MediaType,
	) -> Result<MediaBytes, RenderError> {
		NodeRenderer::render(
			&mut self.clone(),
			&mut RenderContext::new(entity, world)
				.with_accepts(vec![media_type.clone()]),
		)
	}
}

impl RenderTarget for PlainTextRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }
}

impl RenderTarget for HtmlRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Html] }
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

	/// The media type `accepts` negotiates to under `default`, and the body.
	#[cfg(feature = "bsx")]
	fn negotiate(
		default: MediaType,
		accepts: &[&str],
	) -> Result<(MediaType, String), RenderError> {
		let (mut world, entity) = page();
		let accepts = accepts
			.iter()
			.map(|accept| MediaType::from(*accept))
			.collect::<Vec<_>>();
		RenderTargets::render_negotiated(&mut world, entity, &accepts, &default)
			.map(|bytes| (bytes.media_type().clone(), bytes.to_string()))
	}

	/// An empty `accepts` renders the default media type.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn empty_accepts_renders_the_default() {
		negotiate(MediaType::Html, &[])
			.unwrap()
			.xpect_eq((MediaType::Html, "<div><p>hi</p></div>".to_string()));
		negotiate(MediaType::Text, &[])
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
	}

	/// `accepts` is consulted in priority order, the first registered type
	/// winning, plain text an ordinary target like any other.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn accepts_in_priority_order() {
		negotiate(MediaType::Text, &["text/html", "text/plain"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
		negotiate(MediaType::Html, &["text/markdown", "text/html"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Markdown);
		negotiate(MediaType::Html, &["text/plain"])
			.unwrap()
			.xpect_eq((MediaType::Text, "hi\n".to_string()));
		negotiate(MediaType::Html, &["text/plain", "text/html"])
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
		negotiate(MediaType::Html, &["*/*"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
		negotiate(MediaType::Html, &["text/*"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Html);
		negotiate(MediaType::Markdown, &["image/png", "*/*"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Markdown);
	}

	/// A text type with no target of its own falls back to plain text, even
	/// behind a type nothing renders.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn text_falls_back_to_plain_text() {
		negotiate(MediaType::Html, &["text/css"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
		negotiate(MediaType::Html, &["image/png", "text/csv"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Text);
	}

	/// Nothing renderable is a mismatch naming what was asked for and what is
	/// available.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn no_match_is_a_mismatch() {
		match negotiate(MediaType::Text, &["image/png"]) {
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
		negotiate(MediaType::Html, &["application/json"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Json);
		#[cfg(feature = "postcard")]
		negotiate(MediaType::Html, &["application/x-postcard"])
			.unwrap()
			.0
			.xpect_eq(MediaType::Postcard);
	}

	/// The terminal target answers ansi.
	#[cfg(all(feature = "bsx", feature = "style"))]
	#[beet_core::test]
	fn renders_ansi() {
		negotiate(MediaType::Html, &["text/ansi-term"])
			.unwrap()
			.0
			.xpect_eq(MediaType::AnsiTerm);
	}

	/// A target registered from outside this crate's built-ins answers its own
	/// media type through the same call, and one registered later replaces a
	/// built-in for the type both answer.
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
					cx.accepts[0].clone(),
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
		let shout = MediaType::other("text/x-shout");
		RenderTargets::render(&mut world, entity, &shout)
			.unwrap()
			.to_string()
			.xpect_contains("HI");
		RenderTargets::render(&mut world, entity, &MediaType::Html)
			.unwrap()
			.to_string()
			.xpect_contains("HI");
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
		RenderTargets::render(&mut world, entity, &MediaType::Markdown)
			.unwrap()
			.to_string()
			.trim()
			.xpect_eq("# Title");
	}
}
