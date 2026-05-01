+++
title="The Harvest #10"
created="2026-04-06"
+++

# User Modifiable Users

*Pete Hayman — 1st May, 2026*

<iframe src="https://www.youtube.com/embed/TODO" title="The Harvest #11 - User Modifiable Users" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

> *Computers aren’t the thing. They’re the thing that gets us to the thing.*
> Joe MacMillan — Halt and Catch Fire

Three nights ago I had a few scoops of ice cream for desert. Next night, an even bigger helping, and this afternoon I found myself vegging out on the couch in the middle of the day polishing off the tub to random YouTube videos.

I have this kind of willpower relapse once every few weeks, and have tried many solutions but none have worked out long term. As a software engineer I have a strong sense there is some awesome technical solution to understanding, and eventually tuning, my own behavior.

## Constructionism Everywhere

So what do we know about learning? If you ever meet your favorite band the best question you can ask them is "who's your favorite band?". I love Alan Kay but it was his mentor, Seymour Papert, who really shifted my thinking about the true potential for technology in learning.

As an educator, Papert was less interested in software as an artifact, but as a byproduct of somebody "learning by making", the best way to understand something is to build with it. Active engagement, exploration and agency are far more powerful tools than even the most informative infographic.

Paperts influence is evident in the richness of project based learning we see in STEM today, I think the technological capabilities of the 70s naturally skewed to the hard sciences. But his vision for technology was far broader, seeing it as a general purpose tool for the user to explore and understand any interest.

## A better YouTube or no YouTube?

What would an eating disorder be without engagement-based recommendation algorithms. The knee-jerk reaction to this problem might be to take control of the algorithm by building some local-first, source-agnostic feed aggregator. It certainly seems like a step in the right direction, but could also be a local maxima.

Maybe what I need is better habits in the holistic sense, after staring at a computer screen all day maybe what I need is zero screens during downtime, and I think software could help me understand whats needed to get there.

## Tools for self-understanding

In Wollongong there are two massive museums full of interactive ways to learn about just about any STEM subject, but if I want to understand the causes and treatments for my own addiction the best I'll get is a leaflet with generic bullet points.

We can model human behavior in the general sense just as we can model orbital trajectories. The challenge is that individual problems are deeply individual, and so must be the tooling.

To me this is the greatest potential for malleable software. Tools to help us get wherever we want to go, even when we've used up all our willpower points on playing the patience game with a one-year old who's new favorite joke is to do the exact opposite of what he's asked.


## Beyond Schemas

On the technical side this month I've been exploring self-describing data structures. My first approach was to simply store a schema alongside a value:
```json
// typed_value.json
{
	"value": 42,
	"schema": {
		"type": "number",
	}
}
```

I found this not a great fit for several reasons:
1. Its big: now every value needs an associated schema, creating duplication. We could use the json schema external `$ref` features but that means sourcing the reference file every time we want to check something.
2. Nesting: if I want to get a subfield of the value I also need to walk the schema, clone it, and create a new typed value, resolving local refs etc.
3. Value referencing: we also need to map value references.

The architechture I'm currently exploring is inspired by design tokens, where each value has an associated token key, and each token has an associated type. The resulting structure is flat, small, recursive etc.

```json
{
	// 1. declare the tone
	"io.crates/beet_node/style/material/tones/Primary20": {
		"value": {
			"red": 1.0,
			"green": 1.0,
			"blue": 1.0
		},
		"schema": "io.crates/bevy_color/color/Color"
	},
	// 2. rules modify the token, ie point OnPrimary to the light tone
	"io.crates/beet_node/style/material/themes/DarkScheme": {
		"schema": "io.crates/beet_node/style/rule/Rule",
		"predicate": {
			"Class": "dark-scheme"
		},
		"tokens": {
			"io.crates/beet_node/style/material/colors/OnPrimary": {
				"Token": {
					"document": "Ancestor",
					"key": "io.crates/beet_node/style/material/tones/Primary20",
					"schema": "io.crates/bevy_color/color/Color"
				}
			}
		}
	},
	// 3. components point to whatever the current OnPrimary value is
	"io.crates/beet_node/style/material/rules/ButtonFilled": {
		"schema": "io.crates/beet_node/style/rule/Rule"
		"predicate": {
			"Class": "btn-filled"
		},
		"tokens": {
			"io.crates/beet_node/style/common_props/ForegroundColor": {
				"Token": {
					"document": "Ancestor",
					"key": "io.crates/beet_node/style/material/colors/OnPrimary",
					"schema": "io.crates/bevy_color/color/Color"
				}
			},
		}
	}
}
```

Token schemas and behavior, for instance mapping to css properties, are then registered in the Bevy world.


## Declarative State





I'm currently exploring using this same pattern for state, a typed version of my declarative state proposal from post 5, you can read more about it there.