+++
title = "Guide"
order = 4
+++

# Guide

This page is **markdown** content, not a typed `rsx!` page. The `content`
collection is scanned into a route serving a `BlobScene` that reads this file
from the collection's `BlobStore` and parses it to entities plus frontmatter on
each request.

- the frontmatter `title`/`order` above drives the route's `ArticleMeta`
- there is no content codegen: the markdown is parsed per request
