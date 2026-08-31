+++
title = "Scene Format"
order = 1
+++

# The Scene Format

> 🚧 This page is an early draft, please come and share your thoughts in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).

A scene is serializable data describing the structure and behavior of a tool, and this page describes the format beet runs on and advocates for as a standard for interoperability between malleable applications, just as [standard.site](https://standard.site) unlocked interoperable blogging platforms.

## 1. The shape

A scene is a set of entities, each holding components keyed by identifier, plus resources, which are singletons belonging to the scene as a whole. Relations like `ChildOf` are ordinary components referencing another entity, so hierarchy and any other structure ride the same mechanism.

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

Every value is the serialized form of strongly typed data, which is what keeps a scene both human-editable and machine-checkable.

## 2. Representations

The encoding is not the spec. The same scene can be written as json, BSN, BSX markup or postcard bytes, and an implementation round-trips between them without loss. This page uses json throughout because it is the representation a person is most likely to read and edit.

## 3. Today

Today the format is literally bevy scenes. Identifiers are rust module paths like `bevy_ecs::Name`, the type registry defines what each component deserializes to, and interoperability reaches as far as the bevy ecosystem.

## 4. Where it goes

The work ahead is decoupling the format from any one engine or language.

- **Namespaced identity.** Identifiers move from rust conventions like `module::path` to reverse domain names like `org.bevy.Name`, extensible beyond the cargo ecosystem.
- **Component definitions.** Components gain published definitions in the spirit of an [ATProto lexicon](https://atproto.com/specs/lexicon), where an implementation supports the definitions it cares about and carries the rest untouched, respecting round-trip retention much as the USD spec does.
- **Script interop.** Scenes embed sandboxed scripts, and the spec will formalize how data and methods pass in and out of the sandbox so a script written against one implementation runs on another.

## 5. One implementation

Beet relates to the scene format the way Bluesky relates to ATProto, one view of an open standard that others can implement, fork or replace, and that independence matters more to us than any feature of the engine. If you are building an engine, a client or an adaptor and want to compare notes, come say hi in the [Beet Roomy space](https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm).
