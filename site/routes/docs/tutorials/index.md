+++
title = "Tutorials"
+++

# Tutorials

These lessons take you from an empty project to a working beet tool, one small step at a time. They are meant to be followed in order and typed out as you go, and each one shows a different way a tool built from scenes stays open to change.

Every tutorial assumes a recent nightly Rust toolchain and starts from a fresh binary crate. Each one tells you which beet features to enable as you go.

- [A first behavior](/docs/tutorials/first-behavior) builds behavior as a tree of entities rather than compiled control flow, the openness every later lesson leans on.
- [Speak every interface](/docs/tutorials/every-interface) serves the same routes over the command line and HTTP, showing that the tool is the data and the interface is swappable.
- [Your first agent](/docs/tutorials/first-agent) holds a short conversation with an LLM through the same actor-and-route machinery, so the conversation itself is scene data you can inspect and reshape.

A tutorial on editing a running tool's scene without recompiling is on its way, since that hand-edit is the heart of the gentle slope. Once you have a feel for the moving parts, the [Crates](/docs/crates) section explains how they fit together.

One tutorial does not follow the rules above. [Self-hosted mail](/docs/tutorials/mail) is an infrastructure runbook: it stands up a real mail server on real cloud resources with a real domain, costs about US$43 a month to keep running, and takes a week rather than an afternoon because most of that week is spent waiting on a support case. Read it once you are comfortable with beet, or read it for the parts that are about mail rather than about beet.
