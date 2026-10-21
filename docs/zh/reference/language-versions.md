# 语言版本与特性 gates

`gdfmt_syntax::LanguageVersion`（parser/config 同时重导出）提供 4.3、4.4、4.5、4.6、4.7 五个显式 grammar profiles，默认固定为 4.7。仅表示当前已实现语法的已核验版本差异；不代表完整引擎兼容性。4.8 预览、patch 版本字符串、`latest` 和未知版本均不接受，不会静默回退或联网选择版本。

```rust
use gdfmt_parser::{LanguageVersion, ParseOptions, parse};

let options = ParseOptions {
    language_version: LanguageVersion::Godot4_3,
    ..Default::default()
};
let result = parse("var data: Dictionary[String, int] = {}\n", &options);
assert!(result.diagnostics.iter().any(|d| d.code == "PAR017"));
assert_eq!(result.tree.unwrap().text(), "var data: Dictionary[String, int] = {}\n");
```

配置可设 `"language": { "godotVersion": "4.5" }`。Rust 宿主把 `config.language.godot_version` 传给 `ParseOptions.language_version`；配置发现、逐字段合并、来源追踪和 JSON Schema 使用同一枚举。CLI `lex` 仍只执行独立词法检查。

## 已核验特性表

| 特性                                                | 最早 profile | 证据                                                                                                                                                                                                                                         |
| --------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Dictionary[K, V]`，含声明、返回类型和 as/is        | 4.4          | [4.4 发布说明](https://godotengine.org/releases/4.4/index.html)                                                                                                                                                                              |
| `@export_tool_button`                               | 4.4          | [4.4 beta 1 说明](https://godotengine.org/article/dev-snapshot-godot-4-4-beta-1/)                                                                                                                                                            |
| `@warning_ignore_start` / `@warning_ignore_restore` | 4.4          | [4.4 beta 1 说明](https://godotengine.org/article/dev-snapshot-godot-4-4-beta-1/)                                                                                                                                                            |
| `@abstract` 类与方法                                | 4.5          | [4.5 语言参考](https://docs.godotengine.org/en/4.5/tutorials/scripting/gdscript/gdscript_basics.html#abstract-classes-and-methods)                                                                                                           |
| rest 参数 `...args`，含 static 和 lambda            | 4.5          | [4.5 可变参数函数](https://docs.godotengine.org/en/4.5/tutorials/scripting/gdscript/gdscript_basics.html#variadic-functions)                                                                                                                 |
| `@export_file_path`                                 | 4.5          | 对比 [4.4 parser](https://github.com/godotengine/godot/blob/4.4-stable/modules/gdscript/gdscript_parser.cpp) 与 [4.5 parser](https://github.com/godotengine/godot/blob/4.5-stable/modules/gdscript/gdscript_parser.cpp) 的 annotation 注册表 |
| `@export_multiline` 的 hint 参数                    | 4.6          | 对比 [4.5 parser](https://github.com/godotengine/godot/blob/4.5-stable/modules/gdscript/gdscript_parser.cpp) 与 [4.6 parser](https://github.com/godotengine/godot/blob/4.6-stable/modules/gdscript/gdscript_parser.cpp) 的 annotation 注册表 |

`LanguageFeature::since()` 是运行时特性表，`LanguageVersion::supports()` 查询版本。最早 profile 前报告 `PAR017`，继续完成无损 CST 与其他约束检查；合法性错误仍报告各自的 `PAR` code。独立 `lex` 不接收版本参数，不把识别 token 等同于支持该版本。

测试遍历每个 profile，覆盖最早支持版本前后的正反例、脚本/表达式/lambda、低版本无损恢复、UTF-8 span 和共享诊断预算。manifest 的 `syntaxErrorsByVersion` 一旦出现，必须覆盖全部 profiles；`cargo xtask compatibility` 检查默认 profile 和完整矩阵。缺失或未知 profile 会使验证失败。

类型名、annotation 参数值、继承/abstract 覆盖、运行时行为与完整 4.6/4.7 grammar 差异仍需 analyzer 和进一步引擎覆盖。4.3、4.4、4.5、4.6 stable 与 4.7.2 已接入[固定引擎矩阵](/zh/reference/compatibility)，七个 gates 均有独立正反例。namespace/trait 继续保持不支持状态。
