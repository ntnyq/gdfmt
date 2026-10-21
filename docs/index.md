---
layout: home

hero:
  name: gdfmt
  text: GDScript, consistently formatted.
  tagline: A formatter built in Rust. Set your style, format your project, and keep it consistent in CI.
  image:
    src: /logo.svg
    alt: gdfmt logo
  actions:
    - theme: brand
      text: Get started
      link: /guide/
    - theme: alt
      text: View on GitHub
      link: https://github.com/ntnyq/gdfmt

features:
  - title: Format with confidence
    details: Output is reparsed and checked for syntax and comment preservation. A second pass must produce the same result.
    link: /reference/formatting
    linkText: How formatting works
  - title: Your project's style
    details: Set line width, indentation and newlines. Share settings with JSON configuration, EditorConfig and path overrides.
    link: /guide/configuration
    linkText: Configure gdfmt
  - title: From local edits to CI
    details: Format files, check for differences, preview a diff or send a buffer through stdin. Keep custom layouts with protected regions.
    link: /guide/integrations
    linkText: Set up your workflow
---

## Get started

Build and install from the repository with `cargo install --path crates/gdfmt_cli --locked`, then run in your Godot project:

```sh
gdfmt .
gdfmt --check .
```

See the [quickstart](/guide/) for installation. The npm package is not yet published.

## Explore the docs

[Configuration](/guide/configuration) · [CLI reference](/reference/cli) · [Language support](/reference/compatibility) · [Node.js API](/reference/node)
