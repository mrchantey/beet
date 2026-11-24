# Beetmania

A fan site for beets, declared entirely in markup and deployed at [beetmania.beet.org](https://beetmania.beet.org): recipes, growing guides, a beet counter, advice on how not to eat too many, and a blog published to Bluesky. `main.bsx` is the entrypoint, `routes/` is the content, `templates/` holds the site's own BSX templates. No Rust authoring, no codegen, and no `main.rs`: the `beet` binary discovers `main.bsx`, and the `CallOnReady` verb it declares runs the command it is asked for the moment the entry loads. One of those commands is `serve`, the site itself; the others deploy it and publish its blog.

```sh
# run from the site dir so the binary discovers its main.bsx (or pass --entry=<path>)
cd examples/beetmania

# HTTP server
beet serve --server=http

# live terminal
beet serve --server=tui

# every command the entry declares
beet --help

# static export to dist/ (a dev command, run from the repo)
beet export-static examples/beetmania

# the browser boots the same entry from the served page (see Layout.bsx),
# on the wasm binary this builds into target/wasm (run from the repo)
just build-wasm-ui
```

Because the site is runtime files, edits to `main.bsx`, the templates, and the routes need no rebuild, just rerun (or re-request, in HTTP mode). Install the CLI with `cargo binstall beet-cli`, or from a checkout `cargo install --path crates/beet-cli` (serving needs only default features).

## Layout

```
beetmania/
  main.bsx       the entrypoint: the commands, the served router and its middleware
  templates/     the documents others are built from, eg Layout.bsx and widgets/Card.bsx
  routes/        the content: every file is a page, eg recipes/borscht.md
  assets/        the beetroot logo, plus what the `assets` command borrows and derives
  derived/       the blog's standard.site index, written by `social/publish`
```

There is no `main.rs`. The `beet` binary discovers `main.bsx`, roots the repo store at this directory (which `<RoutesDir/>` and every document it names resolve against), and loads `main.bsx` as the app root. The root `<CliServer>` then dispatches the command line into its router: `serve` boots the servers, `--server=http` the HTTP listener, `--server=tui` the live terminal, `--server=dom` the page in a browser tab. Static export is a separate dev command, `beet export-static <site-dir>`.

## The served site

The site is the router under the `serve` command:

```html
<Route path="serve" {(HttpServer, TuiServer, SshTuiServer, DomServer)}>
	<Router {(RequestLogger, CacheHeadersMiddleware, CacheHeaders, HelpHandler, NavigateHandler, Layout{document:"templates/Layout.bsx"})}>
		<Instance document="templates/Styles.bsx"/>
		<RoutesDir src="routes"/>
		<ServeBlobs prefix="repo"/>
		<AssetsDir src="wasm" prefix="wasm" cache="1h"/>
	</Router>
</Route>
```

- The servers own the route: `HttpServer`, `TuiServer`, `SshTuiServer` and `DomServer` are transport components, and `--server` selects which boot, so the same markup serves every target with no host binary. A server a binary cannot boot (`SshTuiServer` in a lean build, `HttpServer` in the browser) boots nothing there and the entry keeps its shape.
- `<Router>` is the `beet_router` dispatch component, the servers' child, and a url space of its own: its pages are rooted at `/` however deep the command, and no request reaches a sibling command.
- `<ServeBlobs prefix="repo"/>` publishes the site's own directory read-only at `/repo`, the store the browser process reads this entry through; `<AssetsDir>` mounts the wasm binary the page boots, whose `wasm/beet-ui.{wasm,js}` link to the build output in `target/wasm`.
- `<PackageConfig/>`, beside the commands, is a resource declaration: a capitalized tag naming a `#[reflect(Resource)]` type patches the live resource's named fields (here the site title, description and homepage), leaving the rest, eg the compile-time version, untouched. It produces no markup.
- The `{(..)}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would: request logging, the cache policy (html edge-cacheable, purged by every deploy and `sync`), `--help`, terminal link navigation, and the layout.
- `<Instance document="templates/Styles.bsx"/>` builds another document in place, named by its path: the site's style rules.
- `Layout{document:"templates/Layout.bsx"}` is render middleware: every page's body transcludes into the default `<slot/>` of that document, instantiated around it per request. Declared on a child route instead, it nests inside its ancestors', which is how the blog wears `templates/ArticleLayout.bsx`.
- `<RoutesDir src="routes"/>` scans its directory at spawn time and creates one route per content file (`.md`/`.bsx`/`.html`), served through the shared media-parse pipeline. `index.*` collapses to its directory, and markdown frontmatter is read at scan time so navigation knows every page's title and order.

`templates/Layout.bsx` composes registered widgets (`<RouteHead>`, `<Header>`, `<RouteSidebar/>`, `<Stylesheet/>`) with plain markup. A page instantiates another document by its path, eg `<Instance document="templates/widgets/Card" props={{title:"Today"}}>`, its children filling that document's `<slot/>` (see `routes/counter.bsx`).

## Bindings

All interpolation is reactive and source-prefixed with `@` (the canonical grammar reference is the `beet_core::bsx` module doc). The site uses each source:

- `@doc:` document state: the beet counter binds `{@doc:count=0}` and the buttons mutate it from a one-line event script, `bx:onclick="target.with_field('count', count => count + 1)"`.
- `@res:` resource fields: the footer pulls `@res:PackageConfig.description` straight from the resource (the default document head emits `og:site_name` from `PackageConfig.title` automatically, so the layout no longer hand-writes it).
- `@prop:` document props: `Card.bsx` binds its heading to `{@prop:title}`, filled by the use's `props={{title:"Today"}}`.

HTML responses render with the bindings settled, while live targets (the terminal today) keep syncing them continuously.

The same site renders on the web (a full HTML document), in the terminal (charcell with the same style rules), and exports statically, all through the standard router pipeline.

## One counter, three targets

`routes/counter.bsx` is a single no-code page that runs on every target, unchanged:

- **Terminal (in-process):** `beet serve --server=tui` drives the same `@doc:count` and event scripts natively through the document sync, the count repainting in charcell on each click.
- **Web (the browser runtime):** `beet serve --server=http` serves the page as its first paint, and the `<Wasm route="serve"/>` in `templates/Layout.bsx` boots the wasm `beet` binary on this same entry, read through `/repo`, dispatching `serve` under `--server=dom` and opening at the page's own url. The tab then runs the page's world: a click reaches the button's entity through the DOM binding, its `bx:onclick` script runs in the sandboxed iframe backend, and the bound count repaints in place with no network round-trip. Until it boots, the served page is inert.
- **Static export:** `beet export-static examples/beetmania` writes the page with its bindings settled to the initial state, the correct first paint.

There is no framework tier between SSR and the world: below wasm the page is plain HTML and CSS, and above it the browser is one more beet runtime, launched by the page.

## Icons and link previews

`<IconSet src="assets/logo/logo.svg"/>` derives every icon size a browser or a PWA asks for from the one source, and `<SocialCards/>` draws a link preview for every page the sitemap lists. Both are derived files, regenerated only when what they were computed from changes, and `beet check` fails a missing one. The beet logo is the only asset this directory keeps (the harvest post's cover is its largest derived icon); `just beetmania assets` borrows the card font from the workspace's `assets/` (hydrated with `just beet-shared pull`) and derives the rest, and a deploy does the same into the copy it publishes.

## The blog, published

The blog, the Beetmania Almanac, is also a [standard.site](https://standard.site) publication on the `beetmania.beet.org` atproto account: `<StandardSitePub>` declares it, and the `social/publish` command writes one document per dated post to the account's repo, each carrying the post itself as a beet scene, then announces it on Bluesky, where `beetlover.beet.org` reposts and replies. The replies to an announcement are the post's comments, read live in `templates/ArticleLayout.bsx`. Its posts are made up, so the example is where a change to that pipeline is tried before it reaches a real blog; `main.bsx` documents the commands.

## Test accounts

Two Bluesky accounts exist for this example, both made-up characters, their handles, dids and display names written literally in `main.bsx`:

| account | plays | handle | signed up as | did |
| --- | --- | --- | --- | --- |
| Beetmania | a test org: publishes the standard.site documents and announces each post | `beetmania.beet.org` | `test-beetmania.bsky.social` | `did:plc:7bxw2ljdoqcny4rio7oiqsru` |
| Beet Lover | a test user: reposts the announcements it is credited on and replies, the comments under each post | `beetlover.beet.org` | `test-beetlover.bsky.social` | `did:plc:xfhml5eyathkdgloyiresv4d` |

Each `beet.org` handle is a `_atproto` TXT record the `social` stack publishes (`just beetmania social deploy --stage=prod`), claimed once in the account's settings (Handle, "I have my own domain"); `just beetmania social probe --stage=prod` checks both. Their app passwords are `BEETMANIA_BEET_ORG_APP_PASSWORD` and `BEETLOVER_BEET_ORG_APP_PASSWORD` in the `agents` group of this example's own `secrets.toml` (see [The secrets](#the-secrets)), rolled at `https://bsky.app/settings/app-passwords`. The repo's `secrets.toml` keeps a copy of each in its own `agents` group for the live tests, so a roll restates both.

Beetmania is also the repo's live test account: `just test-atproto-live` writes to it, only records it deletes again under its own collection and an `https://example.com` publication, so this site's publication is never touched.

## The deploy

`main.bsx` declares its own infrastructure: a Lambda function serving this directory out of an S3 bucket at `beetmania.beet.org`, and the atproto handle records for its two accounts. Both publish only from the production stage, so the example deploys as `--stage=prod`:

```sh
just beetmania deploy --stage=prod
just beetmania social deploy --stage=prod
```

The deploy machinery sits behind `bx:cfg`, so a lean binary loads the same entry, serves the site and leaves the deploy commands out.

A first deploy, or one after the declarations change what a deploy credential needs, runs elevated (`just beetmania deploy --stage=prod --elevated`), which asks the operator for the phone code and the Cloudflare mint token's roll and converges `beetmania-deploy` at both providers, lowered from this entry's stacks alone. `just beetmania audit --stage=prod` then checks the `beet.org` zone holds exactly what this entry declares under its own names.

## The secrets

A package owns its secrets document: this example's is `secrets.toml` beside `main.bsx`, which the entry's `<Secrets/>` loads and its `secrets` verbs address (`just beetmania secrets/check`, `secrets/ls`, `secrets/set`). Its `default` group holds `TF_STATE_PASSPHRASE`, the encryption of both stacks' state, and the deploy credentials an elevated deploy seals; its `agents` group the two app passwords. A secrets document holding a deploy credential belongs to exactly one entry, so nothing the example holds reaches the site's resources, and a launch of another entry against this document is refused.
