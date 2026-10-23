# 语言支持与限制

gdfmt 格式化 UTF-8 编码的 GDScript `.gd` 文件，提供 Godot **4.3、4.4、4.5、4.6 和 4.7** 语法配置，默认 **4.7**。请选择与你的项目对应的版本：

```json
{
  "language": { "godotVersion": "4.5" }
}
```

这些配置检查已核验的语法版本差异，不代表与某个 Godot 版本完整兼容。不接受 patch 版本字符串、`latest`、Godot 3.x 和未知版本。特性引入边界见[语言版本](/zh/reference/language-versions)。

## 格式化支持

覆盖表达式、调用、数组与字典、注解、声明、函数与 lambda、属性、控制流语句块和 match 模式。注释保留原文、顺序与附着关系，多行字面量内容及[保护区域](/zh/reference/formatting#protected-regions)逐字节保留。

格式化器会重新解析输出、比较语法和注释，并验证幂等性。输入语法错误或输出验证失败时，不提供可写结果。这些检查保护格式化契约，不能证明运行时等价。

## 已知边界

| 范围           | 限制                                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------------------ |
| 项目语义       | 类型与名称解析、继承、abstract 覆盖、资源存在性和运行时行为由 Godot 负责                                     |
| 注解值         | 检查名称、目标、参数个数与部分字面量限制，不完整分析一般参数求值、export/RPC 值和警告名称目录                |
| Unicode 标识符 | 使用固定的 Godot Unicode XID 表，但未复现引擎 TextServer 的易混淆关键字安全检查                              |
| 保留构造       | 不支持 `namespace`、`trait` 声明；识别 token 不代表支持对应语言特性                                          |
| 格式选项       | 不提供引号转换、尾逗号增删选项，保留字面量词形与已有标点                                                     |
| 行宽           | 仅在受支持断点上尝试满足目标宽度；长表达式/调用链、不可拆分的 token/类型及保留的注释/字面量/保护区域可能超宽 |
| 文件类型       | 不格式化 Godot 场景、资源文件、着色器或内嵌 GDScript                                                         |
| 编辑器工具     | 提供仅用于格式化的 stdio LSP 与 VS Code 桌面/Web 扩展；范围格式化与项目分析不在当前编辑器范围内              |

lint 规则与语义自动修复不属于格式化器范围。`gdfmt check` 成功仅说明语法和格式检查通过，项目仍应通过 Godot 验证。

桌面设置与 Web 适配器的配置限制见[编辑器集成](/zh/guide/integrations#vs-code)。

## 从 gdformat 迁移

gdfmt 使用自己的布局规则，不是 gdtoolkit 的 `gdformat` 的风格兼容替代品。有意保留的差异包括字面量词形、标点和注释附着关系，配置文件也不能互换。

采用 gdfmt 时，先运行 `gdfmt --diff .`，单独审查格式化改动，提交 `.gdfmtrc.json`，并在本地与 CI 使用同一 formatter revision。输出有差异时，避免在相同文件上交替运行多个格式化器。

## 兼容性测试

仓库保留原创用例、按版本分类的引擎预期、语法清单和固定项目试用。引擎测试通过精确版本的 Godot 二进制，以 `--check-only` 检查原始与格式化脚本，不执行脚本。语义分析失败与语法失败单独分类。

复现命令和引擎版本见[贡献指南](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#engine-comparisons)，机器可读记录保留在 [`tests/compatibility`](https://github.com/ntnyq/gdfmt/tree/main/tests/compatibility) 和 [`tests/project-trials`](https://github.com/ntnyq/gdfmt/tree/main/tests/project-trials)。

[0.1.0 前完整项目试用](../research/0.1.0-project-trials-review.md)在 4.7 profile 下
复查了 606 个固定脚本，2,420 种有效布局全部通过，606 个审查指纹均未改变，
没有未解释差异。这份工作区证据不代表项目导入/运行时行为已验证，也不能替代
最终候选版本验收。
