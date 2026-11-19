+++
title = "An Example Post With Everything"
description = "A made-up post from the beet example site, carrying every kind of block the publishing pipeline handles, so one post exercises them all."
tags = ["examples", "markup"]
created = "2025-03-14"
authors = ["Beet Example", "Beet Test"]
image_url = "/assets/screenshots/hello-world.png"

[RouteMeta]
slug = "an-example-post-with-everything"
+++

This post is an example. Nothing here happened: it exists so the beet example site has something to publish, and every kind of block a post can hold appears in it at least once.

## A heading, then paragraphs

A paragraph is the plainest block. This one has **strong** text, *emphasis*, `inline code` and a [link to beet](https://beet.org), so the inline marks are covered too.

A second paragraph follows the first, to show the space between them.

### A smaller heading

Code blocks are highlighted by language:

```rust
use beet::prelude::*;

fn main() {
	App::new().add_plugins(BeetPlugins).run();
}
```

## Lists

An unordered list:

- markup pages
- one layout
- a theme

An ordered list:

1. write the post
2. publish it as a document
3. announce it

## A quote

> A made-up quote, attributed to nobody, standing in for the real thing.

## An image

![The hello world example running in a terminal](/assets/screenshots/hello-net.png)

## A table

| block | in this post |
| --- | --- |
| heading | yes |
| code | yes |
| table | this one |
