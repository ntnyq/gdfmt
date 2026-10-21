use gdfmt_parser::{
    LanguageFeature as Feature, LanguageVersion as Version, LexOptions, ParseOptions, parse,
    parse_expression,
};
use gdfmt_syntax::{SyntaxKind as K, ast::SourceFile};
use gdfmt_text::SourceId;

#[test]
fn abstract_declarations_bind_across_comments_and_have_no_suite() {
    for source in [
        "@abstract\nclass_name Base extends RefCounted\n@abstract func execute(value: int) -> void\nfunc concrete(): pass\n",
        "@abstract\n@tool\nextends Node\n@abstract\n# contract\nfunc execute(); @abstract func other() -> int\n",
        "@abstract class Base:\n    @abstract func execute()\n    func concrete(): pass\nclass Other:\n    extends RefCounted\n    pass\n",
        "@abstract\n",
        "class_name Child extends RefCounted\nvar value:\n    get(): return 1\nfunc run(x := 1, ...args: Array): pass\n",
        "var value:\n    get = read_value,\n    set = write_value\n",
    ] {
        let result = parse(source, &ParseOptions::default());
        assert!(
            result.diagnostics.is_empty(),
            "{source:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let root = parse(
        "@abstract class Base:\n    @abstract func execute()\n",
        &ParseOptions::default(),
    )
    .tree
    .unwrap()
    .root();
    let function = root
        .descendants()
        .find(|n| n.kind() == K::FunctionDecl)
        .unwrap();
    let declaration = gdfmt_syntax::ast::Declaration::cast(function).unwrap();
    assert!(declaration.body().is_none());
    assert_eq!(SourceFile::cast(root).unwrap().declarations().count(), 1);
}

#[test]
fn constraints_report_errors_and_preserve_following_declarations() {
    for (source, code) in [
        ("func missing()\n", "PAR004"),
        ("@abstract func f()\n", "PAR016"),
        (
            "@abstract class Base:\n    @abstract func f(): pass\n",
            "PAR016",
        ),
        (
            "@abstract class Base:\n    @abstract static func f()\n",
            "PAR016",
        ),
        (
            "@abstract class Base:\n    @abstract func _init()\n",
            "PAR016",
        ),
        (
            "@abstract\nclass_name Base\nclass Inner:\n    @abstract func f()\n",
            "PAR016",
        ),
        ("func f(a, a): pass\n", "PAR011"),
        ("func f(x):\n    var x = 1\n", "PAR015"),
        ("func f(x):\n    if true:\n        var x = 1\n", "PAR015"),
        ("func f(x):\n    for x in []: pass\n", "PAR015"),
        ("func f(x):\n    match 1:\n        var x: pass\n", "PAR013"),
        ("var value:\n    set(v):\n        var v = 1\n", "PAR015"),
        ("signal changed(a, a)\n", "PAR011"),
        ("func f(...args: Array[int]): pass\n", "PAR011"),
        ("func f(...args: int): pass\n", "PAR011"),
        ("var value: Array[int, float]\n", "PAR008"),
        ("var value: Dictionary[int]\n", "PAR008"),
        ("var value: Other[int]\n", "PAR008"),
        ("var value: Array[Dictionary[String, int]]\n", "PAR008"),
        ("class_name A\nclass_name B\n", "PAR015"),
        ("extends Node\nextends RefCounted\n", "PAR015"),
        ("class_name A extends Node\nextends Node\n", "PAR015"),
        ("var value = 1\nextends Node\n", "PAR015"),
        ("class Inner:\n    class_name Invalid\n", "PAR015"),
        ("class Inner extends Node:\n    extends Node\n", "PAR015"),
        ("var duplicate = 1\nfunc duplicate(): pass\n", "PAR015"),
        ("enum E { A, A }\n", "PAR015"),
        ("enum { A }\nvar A = 2\n", "PAR015"),
        ("func f():\n    var x = 1\n    const x = 2\n", "PAR015"),
        ("@unknown var value = 1\n", "PAR014"),
        ("@abstract var value = 1\n", "PAR014"),
        ("@tool(1)\n", "PAR014"),
        ("@export_range(1) var value = 1\n", "PAR014"),
        ("@export func f(): pass\n", "PAR014"),
        ("func f():\n    @export var value = 1\n", "PAR014"),
        ("@onready static var value = 1\n", "PAR014"),
        ("@rpc static func f(): pass\n", "PAR014"),
        ("@export @export var value = 1\n", "PAR014"),
        ("@export @export_range(0, 1) var value = 1\n", "PAR014"),
        ("extends Node\n@tool\n", "PAR014"),
        ("class_name Base\n@abstract\n", "PAR014"),
        ("class_name Base\n@abstract\nextends Node\n", "PAR015"),
        ("class Inner:\n    @abstract\n    extends Node\n", "PAR015"),
        ("@export_group('A') var value = 1\n", "PAR014"),
        ("@warning_ignore_start(1)\n", "PAR014"),
        (
            "var value:\n    get = read_value\n    set = write_value\n",
            "PAR012",
        ),
        (
            "var value:\n    get: return 1\n    set = write_value\n",
            "PAR012",
        ),
        ("func _static_init(): pass\n", "PAR016"),
        ("static func _static_init(a): pass\n", "PAR016"),
        ("static func _init(): pass\n", "PAR016"),
        ("func _init() -> int: return 1\n", "PAR016"),
        ("func f() -> void: return 1\n", "PAR016"),
        ("static func f():\n    var local = self\n", "PAR016"),
        (
            "static func f(): return func(): return super.f()\n",
            "PAR016",
        ),
        ("static var value = $Child\n", "PAR016"),
        (
            "func f(v):\n    match v:\n        [var x, var x]: pass\n",
            "PAR013",
        ),
    ] {
        let source = format!("{source}\nvar sentinel = 9\n");
        let result = parse(&source, &ParseOptions::default());
        assert!(
            result.diagnostics.iter().any(|d| d.code == code),
            "{source:?}: {:?}",
            result.diagnostics
        );
        let tree = result.tree.unwrap();
        assert_eq!(tree.text(), source);
        assert!(
            tree.root()
                .children()
                .any(|n| n.kind() == K::VariableDecl && n.text().to_string().contains("sentinel")),
            "{source:?}"
        );
    }
    for source in ["@export\n", "class_name Base\n@abstract\n"] {
        let result = parse(source, &ParseOptions::default());
        assert!(result.diagnostics.iter().any(|d| d.code == "PAR014"));
    }
}

#[test]
fn class_function_and_lambda_contexts_do_not_leak() {
    for source in [
        "class A:\n    var same = 1\nclass B:\n    var same = 2\n",
        "static func f(): pass\nfunc g(): return self\n",
        "func f():\n    if true:\n        var x = 1\n    else:\n        var x = 2\n",
        "func f(x):\n    var callback = func(x): return x\n",
        "func _init():\n    var callback = func(): return 1\nstatic func _static_init() -> void: return\n",
        "var value:\n    set(v):\n        var callback = func(): return 1\n    get: return value\n",
    ] {
        let result = parse(source, &ParseOptions::default());
        assert!(
            result.diagnostics.is_empty(),
            "{source:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn version_matrix_checks_declarations_expressions_lambdas_and_annotations() {
    let cases = [
        (
            "var value: Dictionary[String, int] = {}\n",
            false,
            Feature::TypedDictionary,
        ),
        (
            "value as Dictionary[String, int]",
            true,
            Feature::TypedDictionary,
        ),
        (
            "value is Dictionary[String, int]",
            true,
            Feature::TypedDictionary,
        ),
        (
            "func f(v: Dictionary[String, int]) -> Dictionary[String, int]: return v\n",
            false,
            Feature::TypedDictionary,
        ),
        (
            "@abstract class Base:\n    @abstract func f()\n",
            false,
            Feature::Abstract,
        ),
        ("func f(...args): pass\n", false, Feature::VariadicFunction),
        (
            "func(...args: Array): return args",
            true,
            Feature::VariadicFunction,
        ),
        (
            "@tool\n@export_tool_button('Run') var action = func(): pass\n",
            false,
            Feature::ExportToolButton,
        ),
        (
            "@warning_ignore_start('unused_variable')\n@warning_ignore_restore('unused_variable')\n",
            false,
            Feature::WarningRegions,
        ),
    ];
    let extra = [
        (
            "@export_file_path('*.txt') var path: String\n",
            false,
            Feature::ExportFilePath,
        ),
        (
            "@export_multiline('language=gdscript') var code: String\n",
            false,
            Feature::MultilineExportHints,
        ),
    ];
    for feature in Feature::ALL {
        assert!(
            cases
                .iter()
                .chain(extra.iter())
                .any(|(_, _, candidate)| *candidate == feature)
        );
    }
    for version in Version::ALL {
        for (source, expression, feature) in cases.into_iter().chain(extra) {
            let options = ParseOptions {
                language_version: version,
                ..Default::default()
            };
            let result = if expression {
                parse_expression(source, &options)
            } else {
                parse(source, &options)
            };
            assert_eq!(
                result.has_errors(),
                version < feature.since(),
                "{version}: {source:?}: {:?}",
                result.diagnostics
            );
            assert!(
                result.diagnostics.iter().all(|d| d.code == "PAR017"),
                "{:?}",
                result.diagnostics
            );
            assert_eq!(result.tree.unwrap().text(), source);
        }
        for source in ["value as Array[int]", "func(x := 1): return x"] {
            assert!(
                !parse_expression(
                    source,
                    &ParseOptions {
                        language_version: version,
                        ..Default::default()
                    }
                )
                .has_errors()
            );
        }
    }
    let source = "@warning_ignore('unused_parameter') func f(v): pass\n";
    for version in Version::ALL {
        let result = parse(
            source,
            &ParseOptions {
                language_version: version,
                ..Default::default()
            },
        );
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
    }
}

#[test]
fn validation_and_version_gates_share_budget_and_utf8_spans() {
    for budget in [0, 1, 3, 4, 100] {
        let source =
            "@abstract\nclass_name 抽象\n@abstract func 方法(...值) -> Dictionary[String, int]\n";
        let options = ParseOptions {
            language_version: Version::Godot4_3,
            lex: LexOptions {
                source_id: SourceId(42),
                max_diagnostics: budget,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = parse(source, &options);
        assert!(!result.diagnostics.is_empty());
        assert!(result.diagnostics.len() <= budget + 1);
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == "PAR002")
                .count(),
            usize::from(budget < 4)
        );
        for diagnostic in &result.diagnostics {
            assert_eq!(diagnostic.source_id, SourceId(42));
            assert!(diagnostic.span.slice(source).is_ok());
        }
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let source = "@abstract\nclass_name 抽象\n@abstract func 方法(...值: Array) -> void\nvar typed: Dictionary[String, int]\n";
    for end in source
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([source.len()])
    {
        let source = &source[..end];
        let result = parse(source, &ParseOptions::default());
        assert_eq!(result.tree.unwrap().text(), source);
        for diagnostic in result.diagnostics {
            assert!(diagnostic.span.slice(source).is_ok());
        }
    }
}

#[test]
fn collection_brackets_in_values_are_subscripts_in_every_profile() {
    for language_version in Version::ALL {
        let options = ParseOptions {
            language_version,
            ..Default::default()
        };
        for source in ["Array[int]([])", "Dictionary[String, int]({})"] {
            let result = parse_expression(source, &options);
            assert!(result.has_errors(), "{language_version}: {source}");
            assert!(!result.diagnostics.iter().any(|d| d.code == "PAR017"));
            assert_eq!(result.tree.unwrap().text(), source);
            let script = format!("var value = {source}\nvar recovered = 1\n");
            let result = parse(&script, &options);
            assert!(result.has_errors());
            assert_eq!(result.tree.unwrap().text(), script);
        }
        for source in ["Array[index]", "Dictionary[key]", "Array[index].call([])"] {
            let result = parse_expression(source, &options);
            assert!(!result.has_errors(), "{source}: {:?}", result.diagnostics);
            assert_eq!(result.tree.unwrap().text(), source);
        }
    }
}

#[test]
fn warning_regions_require_balanced_transitions_in_source_order() {
    for version in Version::ALL {
        let options = ParseOptions {
            language_version: version,
            ..Default::default()
        };
        for source in [
            "@warning_ignore_restore('unused_variable')\n",
            "@warning_ignore_start('unused_variable')\n@warning_ignore_start('UNUSED_VARIABLE')\n",
            "@warning_ignore_start('unused_variable')\n@warning_ignore_restore('unused_variable')\n@warning_ignore_restore('unused_variable')\n",
        ] {
            let result = parse(source, &options);
            assert!(result.has_errors(), "{version}: {source:?}");
            if version >= Version::Godot4_4 {
                assert!(
                    result.diagnostics.iter().any(|d| d.code == "PAR014"),
                    "{:?}",
                    result.diagnostics
                );
            }
            assert_eq!(result.tree.unwrap().text(), source);
        }
        for source in [
            "@warning_ignore_start('unused_variable')\n",
            "@warning_ignore_start('unused_variable')\n@warning_ignore_restore('UNUSED_VARIABLE')\n@warning_ignore_start('unused_variable')\n",
            "@warning_ignore_start('\\u0075nused_variable')\n@warning_ignore_restore(r'UNUSED_VARIABLE')\n",
            "@warning_ignore_start(\"\"\"unused_variable\"\"\")\n@warning_ignore_restore('UNUSED_VARIABLE')\n",
            "func run():\n    @warning_ignore_start('unused_variable')\n    var unused = 1\n@warning_ignore_restore('unused_variable')\n",
        ] {
            let result = parse(source, &options);
            assert_eq!(
                result.has_errors(),
                version < Version::Godot4_4,
                "{version}: {source:?}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.tree.unwrap().text(), source);
        }
    }
}
