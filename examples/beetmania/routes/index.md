+++
title = "Beetmania"
description = "Recipes, growing guides and gentle warnings for people who love beets"
+++

<div bx:style="display=Flex justify-content=Center">
	<section bx:style="display=Flex flex-direction=Vertical align-items=Center text-align=Center max-width=Rem(44.0) row-gap=Rem(1.0)">
		<h1>The home of the humble beet</h1>
		<p class="text-title-large">Recipes, growing guides and a few gentle warnings, for everyone who has ever stained a chopping board purple.</p>
		<Link href="/recipes" variant=ButtonVariant::Filled>Browse the recipes</Link>
	</section>
</div>

## Around the patch

<div class="design-row">
	<section class="card-filled feature-card">
		<h3>Recipes</h3>
		<p>Roasted, pickled, blended or souped: the <a href="/recipes">recipes</a> start simple and stay that way.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Growing</h3>
		<p>Beets are forgiving. The <a href="/guides/growing">growing guide</a> covers sowing, thinning and when to pull them.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>Moderation</h3>
		<p>Yes, there is such a thing as too many beets. Read <a href="/guides/moderation">how not to eat too many</a> before the pink surprise.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>The tally</h3>
		<p>Keep count of today's beets on <a href="/counter">the tally</a>, on the web or in your terminal.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>The almanac</h3>
		<p>News from the patch in the <a href="/blog">almanac</a>, also published to Bluesky by <code>@beetmania.beet.org</code>.</p>
	</section>
	<section class="card-filled feature-card">
		<h3>No code</h3>
		<p>This whole site is markup files, served by beet. See <a href="/about">how it is built</a>.</p>
	</section>
</div>

Beetmania renders as a full HTML document on the web and as charcell in the terminal, from the same files. Run it yourself:

```sh
cd examples/beetmania && beet serve --server=http
```
