//! [`SceneBodyBuilder`]: how a [`SceneBody`] builds into a world.
use crate::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;

/// Builds a [`SceneBody`], the document at a path of its package where it has
/// one, onto the entity of a [`TemplateContext`].
type BuildBodyFn =
	fn(&SceneBody, Option<&RelPath>, &mut TemplateContext) -> Result;

/// How a [`SceneBody`] builds into this world.
///
/// A body is data any crate may hold; reading one against the type registry
/// is the document toolchain's, `beet_scene`, whose `ScenePlugin` registers
/// this. So a component holding a body ([`HeldTemplate`]) and
/// `world.spawn_template(body)` build through it wherever they are written.
#[derive(Clone, Copy, Resource)]
pub struct SceneBodyBuilder(BuildBodyFn);

impl SceneBodyBuilder {
	/// Build every body through `func`.
	pub fn new(func: BuildBodyFn) -> Self { Self(func) }

	/// Build `body`, the document at `path` of its package.
	///
	/// ## Errors
	/// Errors when the world reads no documents, or the body does not build.
	pub fn build(
		body: &SceneBody,
		path: Option<&RelPath>,
		cx: &mut TemplateContext,
	) -> Result {
		let builder =
			*cx.entity.world().get_resource::<Self>().ok_or_else(|| {
				bevyhow!(
					"this world builds no scene body: `beet_scene`'s `ScenePlugin` reads them"
				)
			})?;
		(builder.0)(body, path, cx)
	}
}

/// A body builds as the document it is, its [`DocPath`]s as written: the one
/// way any format reaches a world, `world.spawn_template(body)`.
impl Template for SceneBody {
	type Output = ();

	fn build_template(&self, cx: &mut TemplateContext) -> Result {
		SceneBodyBuilder::build(self, None, cx)
	}

	fn clone_template(&self) -> Self { self.clone() }
}

subtree_template!(SceneBody);
