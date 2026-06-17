+++
title = "Welcome"
description = "A beet site with zero code"
+++

<div bx:style="display=Flex justify-content=Center">
	<section bx:style="display=Flex flex-direction=Vertical align-items=Center text-align=Center max-width=Rem(44.0) row-gap=Rem(1.0)">
		<h1>A site with no code</h1>
		<p class="text-title-large">Pages, layout, theme and styles, all declared in markup. No Rust, no codegen, no build step.</p>
		<Link href="/docs" variant=ButtonVariant::Filled>Read the docs</Link>
	</section>
</div>

## What makes this a no-code site

<div class="design-row">
	<section class="card-filled feature-card">
		<h3>Markup pages</h3>
		<p>Every file under <code>routes/</code> is a page: markdown like this one, or BSX like the <a href="/counter">counter</a>.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>One layout</h3>
		<p>A thin <code>&lt;SiteLayout&gt;</code> in <code>templates/Layout.bsx</code> wraps every route with the header, sidebar and footer.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Brand theme</h3>
		<p>A single <code>&lt;Theme color=.../&gt;</code> in <code>main.bsx</code> sets the accent the whole palette derives from.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Named rules</h3>
		<p>Reusable styles are <code>&lt;Rule&gt;</code> declarations in <code>templates/Styles.bsx</code>, resolved on both targets.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Inline styles</h3>
		<p>A one-off layout is a <code>bx:style</code> on the element, like the hero above, the markup twin of <code>inline_class!</code>.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Checked</h3>
		<p>Run <code>beet check</code> to flag unknown tags, broken links and stray classes before you ship.</p>
	</section>
</div>

The same files render as a full HTML document on the web and as charcell in the terminal. Head over to the [docs](/docs) to see how it fits together, or run this site yourself:

```sh
beet serve examples/bsx_site
```
