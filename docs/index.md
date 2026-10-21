---
layout: home

hero:
  name: gdfmt
  text: Reliable GDScript formatting, built in Rust.
  tagline: A Rust foundation for lossless parsing, structured diagnostics, and reliable formatting.
  image:
    src: /logo.svg
    alt: Godot robot with code brackets and indented lines
  actions:
    - theme: brand
      text: Get started
      link: /guide/
    - theme: alt
      text: Explore the parser
      link: /reference/parsing

features:
  - title: Every byte accounted for
    details: A lossless concrete syntax tree preserves whitespace, comments, and invalid input for the next stage of tooling.
    link: /development/
    linkText: Read the architecture
  - title: Explicit language profiles
    details: Godot 4.3–4.7 profiles gate verified syntax differences, with a pinned engine harness to track compatibility.
    link: /reference/language-versions
    linkText: See the supported scope
  - title: One Rust core
    details: Parsing, configuration, file discovery, and the CLI have separate responsibilities. Script formatting and protected regions use the same validation gates; production configuration and local Node/npm packaging are implemented, with complete artifact acceptance pending CI.
    link: /development/implementation-plan
    linkText: Follow the implementation plan
---

## In development

The lexer, expression and script parsers, configuration schema, and initial CLI are available from source. P3 is accepted with exclusions; script formatting and `check` are available in P4; crates are not published. No Godot version has full compatibility declared.

Start with the [current commands](/guide/), review [compatibility evidence](/development/compatibility), or read the [中文文档](/zh/).
