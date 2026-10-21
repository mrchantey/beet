+++
title = "Standardizing Standard Site Content"
slug = "standardizing-standard-site-content"
description = "The Standard Site 'content' field is swappable but opaque, lets fix that."
tags = ["standard-site", "malleable-software", "atproto"]
created = "2026-10-21"
authors = ["Pete Hayman"]
+++

The unification of longform posts under [Standard Site](https://standard.site) is a fantastic example of the Atmosphere working as intended both culturally and technically, but it has also surfaced where we still have more work to do in primitives for the Atmospheric application layer.

The Standard Site [`content` field](https://lexicon.garden/lexicon/did:plc:re3ebnp5v7ffagz6rb6xfei4/site.standard.document/docs#SiteStandardDocumentMainContent) is opaque to any reader that doesn't speak its format. The lexicon serves to unify discoverability but does not standardize the format of the documents themselves. A reader can extend its supported formats, the Leaflet reader may one day support the `pckt` and `offprint` lexicons, but thats an `O(n^2)` problem with every reader implementing every format despite each describing more or less the same thing.

Lets explore how we might imbue the Standard Site lexicon with a format open to malleability and interop between readers. Below are four variations on the `content` field, defined in typescript here for readability.

## Option 1: Single Format

This is the Standard Site document lexicon as it currently stands.

```ts
type NSID = string;

interface OpenUnion {
  $type: NSID
  [key: string]: unknown
}

interface Document {
  $type: 'site.standard.document'
  // an open content type, ie
  // 'pub.leaflet.content'
  // 'blog.pckt.content'
  // 'app.offprint.content'
  content?: OpenUnion
  //...
}
```

**Example**

```json
{
  "$type": "site.standard.document",
  "content": {
    "$type": "pub.leaflet.content",
    "pages": [
      {
        "$type": "pub.leaflet.pages.linearDocument",
        "blocks": [
          {
            "$type": "pub.leaflet.pages.linearDocument#block",
            "block": {
              "$type": "pub.leaflet.blocks.text",
              "plaintext": "hello world!"
            }
          }
        ]
      }
    ]
  }
  ...
}
```

The great thing about this design is that the `content` field is totally open ended, we can write our post in any format we choose. The challenge is that we can only use one, my post will be Leaflet or Offprint flavored but not both. As an author I want maximum reach but my audience is fragmented.

## Option 2: Multi Format

The least invasive way to fix this is by storing multiple content types in a map keyed by their NSID. The map may contain any arbitrary lexicons so lets call it something generic like `Entity`, also now our `OpenUnion` type has a formal role as only a piece of a larger set so lets rename it to `Component`.

```ts
type NSID = string;

interface Document {
  $type: 'site.standard.document'
  content?: Entity
}

interface Entity {
  $type: 'site.standard.entity'
  // here the key matches the Component $type
  [key: NSID]: Component
}

interface Component {
  $type: NSID
  [key: string]: unknown
}
```

**Example**

```json
{
  "$type": "site.standard.document",
  "content": {
    "$type": "site.standard.entity",
    "pub.leaflet.content": {
        "$type": "pub.leaflet.content",
        ...
    },
    "app.offprint.content": {
        "$type": "app.offprint.content",
        ...
    },
  }
  ...
}
```

Now my blog post will render on leaflet *and* offprint, with each app simply needing to also check for its own lexicon in the NSID map pattern. Now each reader doesn't need to support multiple standards but now have a new problem, writers must know how to render to every format and the content is duplicated, bloating record size.

## Option 3: Standard Components

Lets push this idea further, Leaflet and offprint may not 100% agree on the exact set of components that comprise their documents and will want to innovate freely in the future, but can probably reach consensus on the shape of a Paragraph or Heading. Instead of opaque types lets give `Entity` a `children` field for describing hierarchies, and also define a handful of markup components. The document content becomes the root of a tree, with each heading, paragraph etc as a child entity.

```ts
interface Document {
  $type: 'site.standard.document'
  content?: Entity
}

interface Entity {
  $type: 'site.standard.entity'
  children?: Entity[]
  [key: NSID]: Component
}

interface Component {
  $type: NSID
  [key: string]: unknown
}

// --- COMPONENTS ---

interface Heading {
  $type: 'site.standard.heading'
  text: string
}

interface Paragraph {
  $type: 'site.standard.paragraph'
  text: string
}
// completely extensible, each reader can freely use its own lexicons as components
// in addition to those we have standardized
interface Website {
  $type: 'pub.leaflet.blocks.website'
  src: string
  title?: string
  description?: string
  previewImage?: string
}
```

**Example**

```json
{
  "$type": "site.standard.document",
  "content": {
    "$type": "site.standard.entity",
    "children": [
      {
        "$type": "site.standard.entity",
        "site.standard.heading": {
          "$type": "site.standard.heading",
          "text": "Hello World"
        }
      },
      {
        "$type": "site.standard.entity",
        "site.standard.paragraph": {
          "$type": "site.standard.paragraph",
          "text": "A post dedicated to greeting large realms"
        }
      }
    ]
  }
  ...
}
```

By standardizing the hierarchy we have created an extensible system with fine-grained interoperability. Each reader would need to need to handle these standardized components but thats a tractable task. This shape might be fine if we only wanted to support our 'Big Three' Atmospheric readers, but 55.1% of Standard Site `content` lexicons are markdown/html flavored and 99.5% don't specify a `content` field at all. Also the nested 'children' hierarchy isn't great for upward traversal and other kinds of relations like the `<label for=".."/>` pattern.

## Option 4: HTML Scene Graphs

With just a few tweaks we can achieve html/md compatibility and make our relations parttern extensible. Lets use a generic `Element` type for storing html tags like `h1`, `p` and `b`, which is simpler and more extensible than strongly typed semantics. Lets also refactor the entity tree into a flat list and store the hierarchy in a `Children` component, just like all other per-entity data.

```ts

type EntityId = number

interface Document {
  $type: 'site.standard.document'
  content?: Scene
}

interface Scene {
  $type: 'site.standard.scene'
  [key: EntityId]: Entity
}

interface Entity {
  $type: 'site.standard.entity'
  [key: NSID]: Component
}

interface Component {
  $type: NSID
  [key: string]: unknown
}

// --- COMPONENTS ---

interface Children {
  $type: 'site.standard.children',
  children: EntityId[]
}

// ie a html ElementNode
interface Element {
  $type: 'site.standard.element',
  tag: string
}

// ie a html TextNode
interface Text {
  $type: 'site.standard.text',
  text: string
}
```

**Example**

```json
{
  "$type": "site.standard.document",
  "content": {
    "$type": "site.standard.scene",
    "0": {
      "site.standard.element": {
        "$type": "site.standard.element",
        "tag": "h1",
      },
      "site.standard.children": {
        "$type": "site.standard.children",
        "children": [1]
      },
    },
    "1": {
      "site.standard.text": {
        "$type": "site.standard.text",
        "text": "hello world",
      },
    },
  ...
}
```

And with that our Standard Site is standardized top to tail, extensible and compatible with html. 

## We can just build things

The beauty of the lexicon as it currently stands is *nothing needs to change!* Any reader can implement any one of the above ideas, or not. Option 4 is how Beet works, its the format used by Bevy and other ECS engines and the 'Entity Graph' scene format has proven so simple and generic I'm yet to find a domain it cannot describe elegantly. Stay tuned for a beet standard site reader that works for 99.75% Standard Site documents.

## References

- Related lexicons
	- [pub.leaflet.content](https://lexicon.garden/lexicon/did:plc:btxrwcaeyodrap5mnjw2fvmz/pub.leaflet.content/docs)
	- [blog.pckt.content](https://lexicon.garden/lexicon/did:plc:revjuqmkvrw6fnkxppqtszpv/blog.pckt.content/docs)
	- [app.offprint.content](https://lexicon.garden/lexicon/did:plc:pgjkomf37an4czloay5zeth6/app.offprint.content/docs)
	- [at.markpub.markdown](https://lexicon.garden/lexicon/did:plc:kfxbexqtvw76572grhv2f3on/at.markpub.markdown/docs)
