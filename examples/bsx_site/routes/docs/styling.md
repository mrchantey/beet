+++
title = "Styling"
order = 3
+++

# Styling

Style is semantic classes and design tokens, never raw CSS, so a page renders the same on the web and in the terminal. There are two no-code seams for it: named rules and one-off inline styles.

## Named rules

`templates/Styles.bsx` is a list of `<Rule>` declarations, included once from `main.bsx` via `<Styles/>`. Each rule is the markup analogue of a typed Rust rule: the selector comes from `class`/`tag`/`state`, every other attribute is a declaration in BSX enum form, and a `"@token:Role"` value binds to a Material colour token.

```html
<Rule class={["swatch", "primary"]}
      width=Rem(8.0)
      height=Rem(3.0)
      background-color="@token:Primary"
      color="@token:OnPrimary" />
```

That rule colours these boxes from the theme's `Primary` token, so they track the brand colour and the active colour scheme on both targets:

<div class="design-row">
	<div class="swatch primary">One</div>
	<div class="swatch primary">Two</div>
	<div class="swatch primary">Three</div>
</div>

They sit side by side because a second rule, `design-row`, lays its children out in a wrapping flex row, spaced on the web and the terminal alike.

## Inline styles

For a true one-off, put a `bx:style` directive straight on the element. It is the markup twin of Rust's `inline_class!`: the declarations are the same kebab property names and enum values, registered as a unique rule keyed to that one element.

```html
<section bx:style="display=Flex flex-direction=Vertical align-items=Center max-width=Rem(40.0)">
```

The landing page hero is exactly this: a centered column constrained to a readable measure, declared once with no stylesheet. Note the enum form is `flex-direction=Vertical` (a CSS column), matching the value type rather than the CSS keyword.

## When to use which

Reach for a `<Rule>` whenever a style is shared by more than one element, so the vocabulary lives in one place. Reach for `bx:style` for a layout that belongs to a single element and nowhere else.
