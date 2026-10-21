use gdfmt_parser::{LanguageVersion, ParseOptions, parse};
use gdfmt_syntax::SyntaxKind as K;

fn check(source: &str, rejected: bool) {
    for language_version in LanguageVersion::ALL {
        let result = parse(
            source,
            &ParseOptions {
                language_version,
                ..Default::default()
            },
        );
        assert_eq!(
            result.has_errors(),
            rejected,
            "{language_version}: {source:?}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.tree.unwrap().text(), source);
    }
}

#[test]
fn superclass_names_do_not_accept_collection_type_arguments() {
    check("extends RefCounted\n", false);
    check("extends Array[int]\n", true);
    check("class Inner extends Array[int]: pass\n", true);
}

#[test]
fn semicolons_terminate_statements_but_do_not_form_empty_statements() {
    check("pass; pass\nfunc run(): pass; pass;\n", false);
    check("pass;;pass\n", false);
    check("var value = 1;;\n;\nvar other = 2\n", false);
    check(
        "func run(): pass;; pass;\n\nfunc next():\n    pass;;\n    ;pass\n",
        false,
    );
    for source in [";\n", "func run():\n    ;\n", "@export; var value = 1\n"] {
        check(source, true);
    }
}

#[test]
fn asserts_allow_one_trailing_comma_after_condition_or_message() {
    for arguments in ["true", "true,", "true, 'ok'", "true, 'ok',"] {
        check(&format!("func run(): assert({arguments})\n"), false);
    }
    for arguments in ["", "true,,", "true, 'ok', 1"] {
        check(&format!("func run(): assert({arguments})\n"), true);
    }
}

#[test]
fn standalone_lambdas_are_rejected_through_groups_but_calls_are_valid() {
    for expression in [
        "func(): pass",
        "func named(): pass",
        "(func(): pass)",
        "((func(): pass))",
    ] {
        check(&format!("func run():\n    {expression}\n"), true);
    }
    check("func run():\n    (func(): pass).call()\n", false);
}

#[test]
fn dictionary_rest_allows_later_key_only_matches_as_in_pinned_engines() {
    check(
        "func run(value):\n    match value:\n        {.., 'key'}: pass\n",
        false,
    );
    for pattern in ["{.., 'key': _}", "{.., ..}", "[.., _]"] {
        check(
            &format!("func run(value):\n    match value:\n        {pattern}: pass\n"),
            true,
        );
    }
}

#[test]
fn inline_classes_end_after_one_member() {
    check("class Inner: var value = 1; var value = 2\n", false);
    check("class Inner: extends RefCounted; var value = 1\n", false);
}

#[test]
fn class_comments_and_annotations_follow_class_body_rules() {
    check("@export\npass\nvar value = 1\n", false);
    check("@warning_ignore('unused_variable')\npass\n", true);
    check("'comment'; var value = 1\n", true);
    check(
        "class Inner:\n    'comment'\n    extends RefCounted\n",
        true,
    );
}

#[test]
fn property_setters_allow_return_values_without_an_explicit_void_type() {
    check("var count: int:\n    set(value): return 1\n", false);
    check("func run() -> void: return 1\n", true);
}

#[test]
fn lambdas_see_prior_outer_locals_but_parameters_can_shadow_them() {
    for source in [
        "func run(value):\n    var cb = func():\n        var value = 2\n",
        "func run():\n    var outer = 1\n    var cb = func():\n        var outer = 2\n",
        "func run():\n    for item in []:\n        var cb = func():\n            var item = 1\n",
        "func run(value):\n    match value:\n        var entry when (func():\n            var entry = 1\n            return entry\n        ).call(): pass\n",
        "func run(value = 1, cb = func():\n    var value = 2\n    return value\n): pass\n",
    ] {
        check(source, true);
    }
    for source in [
        "func run():\n    var outer = 1\n    var cb = func(outer): return outer\n",
        "func run():\n    var cb = func():\n        var cb = 1\n    var outer = 2\n",
        "func run():\n    var cb = func():\n        var outer = 1\n    var outer = 2\n",
    ] {
        check(source, false);
    }
}

#[test]
fn malformed_property_accessors_recover_within_the_property_block() {
    let source = "var broken: int:\n    unknown: pass\n    get: return 1\nvar sentinel = 1\nfunc after(): pass\n";
    for language_version in LanguageVersion::ALL {
        let result = parse(
            source,
            &ParseOptions {
                language_version,
                ..Default::default()
            },
        );
        assert!(result.has_errors());
        let tree = result.tree.unwrap();
        assert_eq!(tree.text(), source);
        let first = tree.root().first_child().unwrap();
        assert_eq!(first.kind(), K::VariableDecl);
        assert!(first.descendants().any(|node| node.kind() == K::ErrorNode));
        assert!(first.descendants().any(|node| node.kind() == K::Getter));
        let kinds: Vec<_> = tree.root().children().map(|node| node.kind()).collect();
        assert_eq!(kinds, [K::VariableDecl, K::VariableDecl, K::FunctionDecl]);
    }
}

#[test]
fn official_identifier_attribute_and_empty_match_forms_are_accepted() {
    check("var PI = 1\nvar TAU = 2\nvar INF = 3\nvar NAN = 4\n", false);
    check(
        "func run():\n    var data = {}\n    data.if = 1\n    data.return = 2\n    data.class = 3\n    data._ = 4\n",
        false,
    );
    check("func run():\n    match 0:\n        pass\n", false);
}

#[test]
fn expanded_identifier_forms_still_obey_scopes_and_attribute_boundaries() {
    check("var PI = 1\nvar PI = 2\n", true);
    check("func run(PI):\n    var PI = 1\n", true);
    check(
        "func run():\n    for TAU in []:\n        var TAU = 1\n",
        true,
    );
    for attribute in ["true", "false", "null", "1"] {
        check(
            &format!("func run():\n    var data = {{}}\n    data.{attribute} = 1\n"),
            true,
        );
    }
}

#[test]
fn match_branch_annotations_and_pass_follow_their_special_grammar() {
    check(
        "func run(value):\n    match value:\n        @warning_ignore('unused_variable')\n        pass\n        _: var unused = 1\n",
        false,
    );
    for body in [
        "pass;\n",
        "@warning_ignore('unused_variable')\n        pass\n",
        "@export_group('x')\n        _: pass\n",
        "@export\n        _: pass\n",
    ] {
        check(
            &format!("func run(value):\n    match value:\n        {body}"),
            true,
        );
    }
}

#[test]
fn icon_requires_a_nonempty_string_literal_without_expression_evaluation() {
    for value in ["1", "'res://' + 'x.svg'", "''", "r''", "'\\\n'"] {
        check(&format!("@icon({value})\nextends RefCounted\n"), true);
    }
    check("@icon('res://script.svg')\nextends RefCounted\n", false);
}

#[test]
fn void_returns_allow_calls_and_enclosed_lambda_bodies_end_at_delimiters() {
    for source in [
        "func noop() -> void: pass\nfunc run() -> void: return noop()\n",
        "func run() -> void: return (str(1))\n",
        "func run():\n\taccept(\n\t\tfunc():\n\t\t\tpass)\n\tvar after = 1\n",
        "func run():\n\taccept(\n\t\tfunc():\n\t\t\tpass, 1)\n\tvar after = 1\n",
    ] {
        check(source, false);
        if source.contains("var after") {
            let tree = parse(source, &ParseOptions::default()).tree.unwrap();
            let function = tree
                .root()
                .children()
                .find(|node| node.kind() == K::FunctionDecl)
                .unwrap();
            let suite = function
                .children()
                .find(|node| node.kind() == K::Suite)
                .unwrap();
            assert_eq!(
                suite.children().map(|node| node.kind()).collect::<Vec<_>>(),
                vec![K::ExpressionStmt, K::VariableDecl]
            );
        }
    }
    check("func _init(): return str(1)\n", true);
}
