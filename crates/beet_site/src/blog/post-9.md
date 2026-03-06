+++
title="The Harvest #9"
created="2026-03-06"
+++

# The Harvest #9 - Multi-Interface Applications

<iframe src="https://www.youtube.com/embed/MIlRSPAZ1Fo" title="The Harvest #9 - Multi-Interface Applications" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

Last month I [began exploring](/blog/post-8) an agnostic `Request/Response` pattern so I could use the same router library for both a cli and a server.


**Request**

| type| server | cli |
|---|---|---|
| `Vec<String>` | path | positional args |
| `HashMap<String,Vec<String>>` | query params | options/flags |

This is how [beetstack.dev works currently](https://github.com/mrchantey/beet/blob/83c8feb5ce60b64417baf97c0f2000c5cda09d5e/crates/beet_build/src/actions/beet_cli.rs#L14), the cli that builds the server and static html uses the same router as the server itself. A single project often contains several applications and using the same primitives reduces complexity.

But what happens if you go the other way? A single application with multiple interfaces?

## Prior Art - Content Negotiation

Ever heard of [content negotiation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation
)? I'm embarresed to say that I only discovered it last week. The original vision for the web was so cool! Servers that treat clients as individuals who maintain maximum control, deciding how they'd like their content delivered:

```sh
# browser wants html

GET /users/42
User-Agent: Mozilla/5.0
Accept: text/html

# api wants json

GET /users/42
User-Agent: curl/8.0
Accept: application/json
```

All kinds of `Accept-` headers exist, even for spoken languages!

![HTTP Content Negotiation Diagram](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Content_negotiation/httpnego.png)

Content negotiation fell out of favor for a [variety of reasons](https://wiki.whatwg.org/wiki/Why_not_conneg), but the rise of clankers seems to have driven a bit of a resurgence. Wordpress [recently announced](https://make.wordpress.org/meta/2026/03/03/markdown-now-available-on-wordpress-org/) support for the `Accept: text/markdown` header in their api docs:
```sh
curl -sH 'Accept: text/markdown' https://developer.wordpress.org/reference/functions/get_permalink
```

## Beyond HTML

The above example demonstrates serving either the raw data (json) or a html view (html/md), but what if we took this idea a little further?

Seeing as terminals are so hot right now, maybe you'd like to build a TUI alongside your website. Or maybe you'd like to provide the view as a json tree structure for a game engine ui. Usually that means writing two entirely seperate frontends, but it sounds like the kind of thing a framework should be able to do.

```rust
fn render_page(request: Request, page: Page) -> Response {
	match request.header("User-Agent"){
		"Mozilla/5.0"     => into_html(page),
		"openclaw/2026.3" => into_markdown(page),
		"curl/8.0"        => into_ansi(page),
		"bevy/0.18"       => into_scene(page),
	}
}
```

## Beyond HTTP

Seeing as the Request/Response types are already agnostic to the transport we can start getting weird, for instance instead of sending the payload as a response body treating the request as an instruction to update an in-process persistent interface. 

Take ThePrimeagean's very cool `ssh terminal.shop` idea. Currently visiting `terminal.shop` in the browser simply tells you to go to your terminal, but with this technique a single server could be used to ssh into as a TUI, and also serve *that exact same application* as a web app. Heck while we're at it why not also spin up a native app using `bevy_ui`!

Write once, deploy everywhere, but like this time for realsies.
