---
# for parser whatever it is for the rest of that line, trimming whitespace, as a string. people can parse it as they wish later.
title: Sweet


---


Rusty tools for developing reative structures.

Sweet builds upon Astro's principle of creating interoperability between various reactive libraries, and extends that capability with support for multiple renderers and authoring flavours.

# Planned Authoring Flavours
- Vanilla Rust `RsxNode`
- Reactive html `rsx!`
- Reactive bevy scenes `bsn!`

# Reactive frameworks
- [Leptos Signals](https://crates.io/crates/reactive_graph) 
- Headless Bevy

# Planned Renderers
- html
- bevy graphics
- esp32 robotics
- agentic ai
- behavior trees

## The preprocessor

Sweet has a preprocessor that enables instant reloads for non-code changes in `rsx!` macros.



# Features
- 🔥 **Smokin hot reload** instant reloads for non-code changes
- 🌊 **Stay Hydrated** sweet collects pre-hydration events and plays them back in order, no missed events!
-  **Scoped CSS** with component `<style/>` tags
- 🌐 **Great signal here!** sweet provides integrations with leptos.
- 🦀 **Rusty to the core** components are described as *regular structs and traits*.
- 🧪 **A full ecosystem** sweet has a built-in component library and testing framework, as well as integrations with axum, leptos and bevy.

## Choose your own adventure
- [Quick Start - Counter](./quickstart.md)
- [How it works](./how-it-works.md)