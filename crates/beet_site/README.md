# beet_site

The [beet website](https://beetstack.dev), built with beet.

Pages are authored as `beet_ui` body scenes and rendered to two targets, selected at the edge: a web document shell over HTTP, and the charcell terminal renderer. The `src/codegen` route modules are generated from the content tree by `run_codegen` (the `codegen` feature) before a `web` or `terminal` build.

This crate doubles as the reference example for building a real beet app.
