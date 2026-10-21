---
layout: home

hero:
  name: gdfmt
  text: 以 Rust 构建可靠的 GDScript 格式化器。
  tagline: 以 Rust 构建无损解析、结构化诊断与可靠格式化的共同基础。
  image:
    src: /logo.svg
    alt: Godot 机器人与代码括号、缩进线条
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/
    - theme: alt
      text: 了解解析器
      link: /zh/reference/parsing

features:
  - title: 保留每一个字节
    details: 无损具体语法树保留空白、注释与非法输入，为后续工具提供完整源码。
    link: /zh/development/
    linkText: 阅读架构说明
  - title: 显式语言版本
    details: Godot 4.3–4.7 profiles 对已核验的语法差异进行版本检查，固定引擎测试持续记录兼容性。
    link: /zh/reference/language-versions
    linkText: 查看支持范围
  - title: 共享 Rust 核心
    details: 解析、配置、文件发现与 CLI 各有明确职责。脚本格式化和保护区域使用相同验证门禁；生产配置与本地 Node/npm 打包已实现，完整产物验收待 CI。
    link: /zh/development/implementation-plan
    linkText: 查看实施计划
---

## 开发状态

目前可从源码使用词法分析、表达式与脚本解析、配置 Schema 和基础 CLI。P3 已按排除项验收，脚本格式化与 `check` 已进入 P4，crates 暂未发布，尚未声明任何 Godot 版本的完整兼容性。

从[当前命令](/zh/guide/)开始，查看[兼容性证据](/zh/development/compatibility)，或阅读 [English documentation](/)。
