+++
title = "Scenes"
order = 1
+++

# Scenes

A scene is beet's unit of everything. It describes structure and behavior as data, it is the record a repo is made of, and it is the layer of malleability most people touch first, since changing a scene changes the software while it runs. This page is about what a scene is and why beet works from nothing else. [A guestbook from a scene](/docs/tutorials/guestbook-scenes) builds one.

## The shape

A scene is a set of entities, each holding components keyed by identifier, plus resources, which are singletons belonging to the scene as a whole. Relations like `ChildOf` are ordinary components referencing another entity, so hierarchy and every other structure ride the same mechanism.

```json
{
	"resources": {
		"bevy_ecs::Time": 393933892
	},
	"entities": {
		"0": {
			"bevy_ecs::Name": "Bob"
		},
		"1": {
			"bevy_ecs::ChildOf": "0"
		}
	}
}
```

Every value is the serialized form of strongly typed data, which keeps a scene both human-editable and machine-checkable. Entity keys stay stable across saves and file order is child order, so a scene diffs and merges the way a document does.

## Representations

The encoding is not the scene. JSON is the record, the form a repo stores and beet edits at the component grain. BSX markup and markdown are ways of writing one for a person, imported into the record rather than kept beside it, because text is easy to lose structure in. The guestbook's `main.bsx` is a scene written in BSX:

```jsx
<CallOnReady {CliServer}>
<Router>
	<Route path="/" {FixedPage}>
		<h1>Guestbook</h1>
		<p>Nobody has signed yet.</p>
	</Route>
</Router>
</CallOnReady>
```

Each tag is an entity, each attribute or `{spread}` a component and each nested tag a child. Written as JSON it is the same scene, and a runtime treats the two identically.

## Scenes all the way down

Everything in a repo is a scene or a plain document, apart from blobs. The main scene is one, and so is a route, a page, a behavior tree, a thread of agents, a deploy stack and a [script](/docs/scripts). A reusable scene is a record whose root is named and instantiated by cloning, so a layout or a widget is a scene the same way a page is.

Because behavior lives in scenes rather than compiled control flow, a scene stays open while it runs. Editing one, in a text editor, in the scene editor on any surface or through an agent, is an ordinary edit to a record, and the running software follows it component by component with no rebuild and no redeploy. The words a scene may use come from the runtime, and when the vocabulary runs out the next layers are [scripts](/docs/scripts) and [plugins](/docs/plugins).

## A draft standard

> 🚧 This section is an early draft, please come and share your thoughts in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).

Today the format is Bevy scenes. Identifiers are rust module paths like `bevy_ecs::Name`, the type registry defines what each component deserializes to, and interoperability reaches as far as the Bevy ecosystem. The work ahead is decoupling the format from any one engine or language, just as [standard.site](https://standard.site) unlocked interoperable blogging platforms.

- **Namespaced identity.** Identifiers move from rust conventions like `module::path` to reverse domain names like `org.bevy.Name`, extensible beyond the cargo ecosystem.
- **Component definitions.** Components gain published definitions in the spirit of an [ATProto lexicon](https://atproto.com/specs/lexicon), where an implementation supports the definitions it cares about and carries the rest untouched, respecting round-trip retention much as the USD spec does.
- **Script interop.** Scenes embed sandboxed scripts, and the spec will formalize how data and methods pass in and out of the sandbox so a script written against one implementation runs on another.

Beet relates to the scene format the way Bluesky relates to ATProto, one view of an open standard that others can implement, fork or replace, and that independence matters more to us than any feature of the engine. If you are building an engine, a client or an adaptor and want to compare notes, come say hi in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).
