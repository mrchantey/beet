//! [`ToggleSceneEditor`] and [`SceneEditor`]: the scene editor's frame, and
//! the seam binding it to the scene it edits.
use super::style::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// Edit mode for the scene this tag sits in: a [`SceneEditor`] behind a closed
/// disclosure, so a page ships the ability to edit itself without wearing it.
///
/// A reflected component rather than a template, because the tag must survive
/// the scene's fork: the document keeps `<ToggleSceneEditor/>` as one entity
/// among the page's, where an inspector can select it and removing it is an
/// ordinary edit, and every boot from the fork rebuilds the editor from it.
/// The widgets themselves are spawned under it on add as [`Derived`]
/// editor ui, which no dump ever includes, so a fork holds the tag and never
/// the tree it generated.
///
/// ```html
/// <main>
///   <h1>Garden</h1>
///   <ToggleSceneEditor/>
/// </main>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(spawn_editor_ui))]
pub struct ToggleSceneEditor {
	/// The disclosure's label.
	pub label: SmolStr,
}

impl Default for ToggleSceneEditor {
	fn default() -> Self {
		Self {
			label: "Scene editor".into(),
		}
	}
}

/// The editor ui a [`ToggleSceneEditor`] spawns: the disclosure and the
/// editor, derived from the tag and rebuilt from it on every boot.
fn spawn_editor_ui(
	toggle: &ToggleSceneEditor,
) -> impl FnOnce(&mut EntityCommands) + use<> {
	let label = toggle.label.to_string();
	move |entity: &mut EntityCommands| {
		let tag = entity.id();
		entity
			.commands()
			.spawn((Derived, ChildOf(tag)))
			.insert_template(rsx! {
				<details {Classes::new([SCENE_EDITOR_TOGGLE])}>
					<summary>{label}</summary>
					<SceneEditor/>
				</details>
			});
	}
}

/// The scene editor: the [`SceneTree`] beside the [`EntityInspector`], bound
/// to the nearest scene document.
///
/// Resolution is by ancestry, as every document binding's is: an ancestor
/// [`DocRef`] names the scene explicitly, else the nearest ancestor carrying a
/// [`SceneDocument`] is the scene this editor sits inside. Either way the
/// editor pins what it found with a `DocRef` of its own
/// ([`link_scene_editors`]), so a document declared nearer later never
/// captures its bindings, and a scene that has not arrived (a store still
/// reading) leaves it unbound until it does.
///
/// ```rsx
/// <Fragment bx:ref="scene" {SceneBlob::new("app.json", "app.bsx")}/>
/// <SceneEditor {DocRef($scene)}/>
/// ```
#[template]
pub fn SceneEditor() -> impl Bundle {
	rsx! {
		<div {(SceneEditorRoot, Classes::new([SCENE_EDITOR]))}>
			<div {Classes::new([SCENE_EDITOR_TREE])}>
				<SceneTree/>
			</div>
			<div {Classes::new([SCENE_EDITOR_INSPECTOR])}>
				<EntityInspector/>
			</div>
		</div>
	}
}

/// Marks a [`SceneEditor`]'s root, the entity its [`DocRef`] to the scene
/// lands on.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct SceneEditorRoot;

/// The entity a scene editor has selected, by file key, on the scene
/// document's host: per document, so two editors over one scene agree and
/// neither reaches another.
///
/// Written by the [`SceneTree`]'s rows, read by the [`EntityInspector`], and
/// `None` until something is selected or when the selected entity was removed
/// from the document.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct SceneSelection(pub Option<u32>);

/// System: bind each unbound editor to its scene, and give the scene a
/// selection.
///
/// A scene arrives late as often as not (a `SceneBlob` reads its store off a
/// task), so an editor that resolves no scene this frame simply tries again
/// next frame; nothing is reported until a scene is there to bind.
pub(super) fn link_scene_editors(
	editors: Populated<Entity, (With<SceneEditorRoot>, Without<DocRef>)>,
	doc_refs: AncestorQuery<&DocRef>,
	scenes: AncestorQuery<Entity, With<SceneDocument>>,
	is_scene: Query<(), With<SceneDocument>>,
	selections: Query<(), With<SceneSelection>>,
	mut commands: Commands,
) {
	for editor in editors.iter() {
		// an authored `DocRef` above the editor names the scene; else the
		// nearest scene document above it is the one it sits inside
		let host = match doc_refs.get_exclusive(editor) {
			Ok(doc_ref) => doc_ref.document(),
			Err(_) => match scenes.get_exclusive(editor) {
				Ok(host) => host,
				Err(_) => continue,
			},
		};
		if !is_scene.contains(host) {
			continue;
		}
		commands.entity(editor).insert(DocRef(host));
		if !selections.contains(host) {
			commands.entity(host).insert(SceneSelection::default());
		}
	}
}

/// System: a selection whose entity the document no longer holds (removed by
/// an edit from outside the editor) clears, so the inspector never binds a
/// form to an entity that is not there.
pub(super) fn clear_stale_selections(
	mut hosts: Populated<(&Document, &mut SceneSelection), Changed<Document>>,
) {
	for (document, mut selection) in hosts.iter_mut() {
		let stale = selection.0.is_some_and(|key| {
			!SceneEntities::of(&document.0)
				.is_ok_and(|entities| entities.contains(key))
		});
		if stale {
			selection.0 = None;
		}
	}
}

/// The scene an editor widget edits: the host its editor's [`DocRef`] names.
///
/// Shared by the tree and the inspector, which are both editor ui under one
/// [`SceneEditorRoot`] and address the scene through it rather than through
/// their own bindings' ancestor walk, so a document nearer than the scene (a
/// form's draft, say) never captures them. The selection is queried beside
/// it, since a widget that writes it holds it mutably.
#[derive(SystemParam)]
pub(super) struct SceneQuery<'w, 's> {
	editors: AncestorQuery<'w, 's, &'static DocRef, With<SceneEditorRoot>>,
}

impl SceneQuery<'_, '_> {
	/// The scene host `widget` belongs to, if its editor is bound.
	pub(super) fn host(&self, widget: Entity) -> Option<Entity> {
		self.editors
			.get(widget)
			.ok()
			.map(|doc_ref| doc_ref.document())
	}
}
