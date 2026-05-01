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
> 
> Joe MacMillan — Halt and Catch Fire


Three nights ago I had a few scoops of ice cream for desert. Next night, an even bigger helping, and this afternoon I found myself vegging out on the couch in the middle of the day polishing off the tub to random YouTube videos.

I have this kind of willpower relapse once every few weeks, and have tried many solutions but none have worked out long term. As a software engineer I have a strong sense there is some awesome technical solution to understanding, and eventually tuning, my own behavior. It begs the question, can technology consistently help us become our favorite version of ourselves?

## Constructionism Everywhere

So what do we know about learning? If you ever meet your favorite band the best question you can ask them is "who's your favorite band?". I love Alan Kay but it's his mentor, Seymour Papert, who really expanded my thinking about the potential for technology.

As an educator, Papert was less interested in software as an artifact, but as a byproduct of somebody "learning by making". Instead of learning the abstract formulas of newtons laws, students are tasked with creating a physics simulation. The abstactions are still there, but in an applied context, and now there's a motivation to learn them in achieving an interesting goal.

Paperts influence is evident in the prevelance of project based learning and other techniques in STEM education, I think the capabilities of computers in the 70s naturally skewed to mathematics and the hard sciences. But his vision for technology was far broader, seeing it as a general purpose tool for the user to gain an active understanding of any interest.

In Wollongong there are two massive interactive museums full of potent techniques for learning about just about any STEM subject, but if I want to understand the causes and treatments for personal addictions the best I'll get is a leaflet with generic bullet points, doing its best to inform most of the people most of the time.

### A Better Video Player?

As a concrete example lets look at the content side of my dopamine stacking habit: engagement based recommendation algorighms. My knee-jerk reaction to this problem is to take control of the algorithm by replacing YouTube with some local-first, [source-agnostic feed aggregator](https://youtu.be/l9CPmPk2R-M?t=935). It certainly seems like a step in the right direction, but could also be treating the symptom, not the cause. Perhaps the deeper issue is some other system or lack thereof in my life, unrelated to the interface itself.

### Tools for a better self

So the problems are definitely holistic, and so must be the solutions. I can think of three steps to an effective solution:

1. **Self-understanding**

Here we need to put on our systems-thinking hats. We can model human behavior in the general sense just as we can model orbital trajectories. The challenge is that individual behaviors are deeply individual, and so must be the tooling.

This is where I believe we can use constructionist principles to explore a behavioral space. The key here is active participatory techniques consistent with known effective social work practices: less passive information and more role-play, simulation, exploratory discussions etc.

2. **The Scientific Method**

Actually addressing behavioral problems is a multi-step process with lots of trial and lots of error. Here I think malleable software is key, even the same problem faced by the same individual may requrire different solutions at different times. The solution should ideally self-modify to suit the user, or at least be easily configured.

3. **Supportive Environment**

This is where the local-first feed aggregator plays an important role, in order to form a habit and keep it we need to remove predatory software from our lives, or it will be waiting for us when willpower is lowest.

### Holistic Solutions

I strongly suspect that for each of these steps to be most effective it must be integrated with the other two. I don't want the video player that makes me want to watch videos all day, I want one that also considers what I could be doing instead and helps me to make a good decision.

To me this is the greatest potential for malleable software. Tools to help us get wherever we want to go, even when we've used up all our willpower points on playing the patience game with a one-year old who's new favorite joke is to do the exact opposite of what he's asked.

## Beyond Schemas

On the technical side this month I've been exploring self-describing data structures. My first approach was to simply store a json schema alongside a value:
```json
// typed_value.json
{
	"value": 42,
	"schema": {
		"type": "number",
	}
}
```

However when actually implementing I found this problematic for a few reasons:
1. Its big: now every value needs an associated schema, creating duplication. We could use the json schema external `$ref` features but that means sourcing the reference file every time we want to validate.
2. Nesting: if I want to get a subfield of the value I also need to walk the schema, clone it, and create a new typed value, rebuilding local refs etc.
3. Value referencing: There's no built-in way to point one value to another.

## Design Tokens

The architechture I'm currently exploring is inspired by design tokens, where each value has an associated token key, and each token has an associated type. The resulting structure is flat, small and recursive.

```json
{
	// 1. declare the tone
	"io.crates/beet_node/style/material/tones/Primary20": {
		"schema": "io.crates/bevy_color/color/Color",
		"value": {
			"red": 0.0,
			"green": 0.2,
			"blue": 0.0
		},
	},
	// 2. rules modify the token, ie point OnPrimary to the light tone
	"io.crates/beet_node/style/material/themes/DarkScheme": {
		"schema": "io.crates/beet_node/style/rule/Rule",
		"predicate": {
			"Class": "dark-scheme"
		},
		"tokens": {
			"io.crates/beet_node/style/material/colors/OnPrimary": {
				"key": "io.crates/beet_node/style/material/tones/Primary20",
				"schema": "io.crates/bevy_color/color/Color"
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
				"key": "io.crates/beet_node/style/material/colors/OnPrimary",
				"schema": "io.crates/bevy_color/color/Color"
			},
		}
	}
}
```

Token keys can be universally unique, using urls or reverse domain name notation, and possibly even versioned for integration with document migration projects like [Cambria](https://www.inkandswitch.com/cambria/). 

I'm currently exploring using this same pattern for state, a typed version of my declarative state proposal from [post 5](https://beetstack.dev/blog/post-5).
