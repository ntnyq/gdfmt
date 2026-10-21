---
layout: home

hero:
  name: gdfmt
  text: 让 GDScript 格式保持一致。
  tagline: 以 Rust 构建的格式化工具。设定风格，格式化项目，在 CI 中保持一致。
  image:
    src: /logo.svg
    alt: gdfmt 标志
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/
    - theme: alt
      text: 在 GitHub 上查看
      link: https://github.com/ntnyq/gdfmt

features:
  - title: 验证每次格式化
    details: 重新解析输出并检查语法结构与注释保留情况，再次格式化必须产生完全一致的结果。
    link: /zh/reference/formatting
    linkText: 了解格式化行为
  - title: 统一项目风格
    details: 配置行宽、缩进与换行，通过 JSON 配置、EditorConfig 和路径覆盖共享项目设置。
    link: /zh/guide/configuration
    linkText: 配置 gdfmt
  - title: 从本地编辑到 CI
    details: 格式化文件、检查差异、预览改动，或通过 stdin 处理缓冲区。保护区域可保留自定义排版。
    link: /zh/guide/integrations
    linkText: 接入工作流
---

## 开始使用

在仓库内运行 `cargo install --path crates/gdfmt_cli --locked` 完成构建与安装，然后在你的 Godot 项目中执行：

```sh
gdfmt .
gdfmt --check .
```

完整安装步骤见[快速开始](/zh/guide/)。npm 预发布版本的包名为 `@gdfmt/gdfmt@next`。

## 浏览文档

[配置](/zh/guide/configuration) · [CLI 参考](/zh/reference/cli) · [语言支持](/zh/reference/compatibility) · [Node.js API](/zh/reference/node)
