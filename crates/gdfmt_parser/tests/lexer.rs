use gdfmt_parser::{LexOptions, lex};
use gdfmt_syntax::{LosslessTree, SyntaxKind as K, Token};
use gdfmt_text::Span;

fn kinds(source: &str) -> Vec<K> {
    let result = lex(source, &LexOptions::default());
    assert_eq!(result.tree.as_ref().unwrap().text(), source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result
        .tokens
        .into_iter()
        .filter(|token| !token.kind.is_trivia())
        .map(|token| token.kind)
        .collect()
}

#[test]
fn indentation_comment_lines_crlf_and_eof_are_lossless() {
    let source = "\u{feff}func test():\r\n\t## doc\r\n\tif true:\r\n\t\tpass";
    let tokens = kinds(source);
    assert_eq!(tokens.iter().filter(|kind| **kind == K::Indent).count(), 2);
    assert_eq!(tokens.iter().filter(|kind| **kind == K::Dedent).count(), 2);
    assert_eq!(tokens.iter().filter(|kind| **kind == K::Newline).count(), 3);
    assert_eq!(tokens.last(), Some(&K::Eof));
    assert_eq!(kinds(""), [K::Eof]);
    assert_eq!(kinds("  # comment\n\n"), [K::Eof]);
}

#[test]
fn continuation_lines_do_not_open_indentation_blocks() {
    let tokens = kinds("var values = [\n  1,\n\t2,\n]\nvar sum = 1 + \\\n    2\n");
    assert!(!tokens.contains(&K::Indent));
    assert_eq!(tokens.iter().filter(|kind| **kind == K::Newline).count(), 2);
}

#[test]
fn literals_and_longest_operators_keep_their_spelling() {
    let source = "var 文本 = r'raw\\n'\nvar name = &\"中😀\"\nvar path = ^'../Child'\nvar text = \"\"\"a\nb\"\"\"\nx **= .5e+2; x <<= 2; x != 0xF_F; x = 0b10\n";
    let tokens = kinds(source);
    for kind in [
        K::String,
        K::StringName,
        K::NodePath,
        K::PowerEqual,
        K::ShiftLeftEqual,
        K::NotEqual,
    ] {
        assert!(tokens.contains(&kind), "missing {kind:?}");
    }
}

#[test]
fn errors_recover_without_discarding_text() {
    for (source, code) in [
        ("var s = 'broken\nvar x = 2\n", "LEX006"),
        ("func f():\n \tpass\n", "LEX004"),
        ("func f():\n    pass\n  pass\n", "LEX005"),
        ("var x = [1)\n", "LEX007"),
        ("var x = (1\n", "LEX008"),
        ("var x = 1e+\n", "LEX011"),
        ("var x = 1.2.3\n", "LEX011"),
        ("var x = 0b10.1\n", "LEX011"),
        ("var x = r'broken\\\nvar next = 2\n", "LEX006"),
        ("var s = '\\q'\n", "LEX012"),
        ("var x = ☃\n", "LEX003"),
    ] {
        let result = lex(source, &LexOptions::default());
        assert_eq!(result.tree.unwrap().text(), source);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{source:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn limits_are_diagnostics_and_green_storage_can_cross_threads() {
    let options = LexOptions {
        max_source_bytes: 2,
        ..Default::default()
    };
    assert!(lex("pass", &options).tree.is_none());
    let options = LexOptions {
        max_nesting: 2,
        max_diagnostics: 2,
        ..Default::default()
    };
    let source = "((((((((????))))))))";
    let result = lex(source, &options);
    assert!(result.has_errors());
    assert!(result.diagnostics.len() <= 3);
    let tree = result.tree.unwrap();
    assert_eq!(
        std::thread::spawn(move || tree.text()).join().unwrap(),
        source
    );
}

#[test]
fn arbitrary_unicode_inputs_always_advance_and_partition_source() {
    let alphabet: Vec<char> = "ab_01  \t\r\n#'\"\\()[]{}@&^.+-?中😀\u{feff}\0\u{301}"
        .chars()
        .collect();
    let mut seed = 0x5eed_u64;
    for length in 0..128 {
        for _ in 0..12 {
            let source: String = (0..length)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    alphabet[(seed >> 32) as usize % alphabet.len()]
                })
                .collect();
            let result = lex(&source, &LexOptions::default());
            assert_eq!(result.tree.unwrap().text(), source);
            let mut cursor = 0;
            for token in result.tokens {
                assert_eq!(token.span.start(), cursor);
                assert!(token.span.slice(&source).is_ok());
                if token.kind.is_virtual() {
                    assert!(token.span.is_empty());
                }
                cursor = token.span.end();
            }
            assert_eq!(cursor as usize, source.len());
        }
    }
}

#[test]
fn cst_builder_rejects_invalid_partitions() {
    let tokens = [Token {
        kind: K::Identifier,
        span: Span::new(1, 2).unwrap(),
    }];
    assert!(LosslessTree::from_tokens("ab", &tokens).is_none());
    let tokens = [Token {
        kind: K::Indent,
        span: Span::new(0, 2).unwrap(),
    }];
    assert!(LosslessTree::from_tokens("ab", &tokens).is_none());
}

#[test]
fn pinned_unicode_ranges_and_reserved_tokens() {
    assert_eq!(
        kinds("_ _value a\u{301} 变量 match when ? ` <<<<<<< ======= >>>>>>>"),
        [
            K::Underscore,
            K::Identifier,
            K::Identifier,
            K::Identifier,
            K::Keyword,
            K::Keyword,
            K::QuestionMark,
            K::Backtick,
            K::VcsConflictMarker,
            K::VcsConflictMarker,
            K::VcsConflictMarker,
            K::Newline,
            K::Eof
        ]
    );
    // U+088F entered XID_Start in Unicode 17; U+1C89 entered in Unicode 16.
    assert_eq!(
        kinds("\u{88f} \u{1c89}"),
        [K::Identifier, K::Identifier, K::Newline, K::Eof]
    );
    for source in ["\u{301}a", "☃", "😀", "\0"] {
        assert!(
            lex(source, &LexOptions::default()).has_errors(),
            "{source:?}"
        );
    }
}

#[test]
fn signed_numbers_follow_operand_context_and_suffixes_keep_boundaries() {
    let source = "-2 + +0xF - -0b10; x-2; - 2; -.5; 1_ 0xF_ 2.e+3 1..2 1...2";
    let result = lex(source, &LexOptions::default());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let numbers: Vec<_> = result
        .tokens
        .iter()
        .filter(|token| token.kind == K::Number)
        .map(|token| token.span.slice(source).unwrap())
        .collect();
    assert_eq!(
        numbers,
        [
            "-2", "+0xF", "-0b10", "2", "2", ".5", "1_", "0xF_", "2.e+3", "1", "2", "1", "2"
        ]
    );
    for source in [
        "0x_1", "0b2", "1__0", "1._0", "1e_2", "1e+", ".5.6", "0x1.2", "12abc",
    ] {
        let result = lex(source, &LexOptions::default());
        assert!(
            result.diagnostics.iter().any(|d| d.code == "LEX011"),
            "{source}"
        );
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let result = lex("12abc", &LexOptions::default());
    assert_eq!(result.tokens[0].span, Span::new(0, 2).unwrap());
    assert_eq!(result.tokens[1].kind, K::Identifier);
}

#[test]
fn strings_allow_physical_newlines_and_validate_unicode_escapes() {
    for source in [
        "'one\ntwo'",
        "r'one\ntwo'",
        "'one\r\ntwo'",
        "'escaped\\\nline'",
        r"'\uD83D\uDE00'",
        r"'\U01F600'",
        r"'\U00D83D\uDE00'",
        r"r'\uD800'",
        "'\\u202e'",
    ] {
        kinds(source);
    }
    for source in [
        r"'\uD800'",
        r"'\uDC00'",
        r"'\uD800x'",
        r"'\uD800\n'",
        r"'\uD800\u0041'",
        r"'\U110000'",
        r"'\u12Z4'",
        "'\u{202e}'",
        "r'\u{2066}'",
    ] {
        let result = lex(source, &LexOptions::default());
        assert!(
            result.diagnostics.iter().any(|d| d.code == "LEX012"),
            "{source:?}"
        );
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let result = lex("'a\nb'\nnext", &LexOptions::default());
    assert_eq!(
        result
            .tokens
            .iter()
            .filter(|t| t.kind == K::PhysicalNewline)
            .count(),
        1
    );
    assert_eq!(
        result
            .tokens
            .iter()
            .filter(|t| t.kind == K::Newline)
            .count(),
        2
    );
}

#[test]
fn explicit_continuation_skips_blank_and_comment_only_lines() {
    let tokens = kinds("var x = 1 + \\\n  # comment\n\n      2\nnext\n");
    assert!(!tokens.contains(&K::Indent));
    assert_eq!(tokens.iter().filter(|kind| **kind == K::Newline).count(), 2);
}
