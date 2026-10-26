use gdfmt_syntax::{GdscriptLanguage, LosslessTree, SyntaxKind as K, directives};
use rowan::{GreenNodeBuilder, Language};

fn tree(parts: &[(K, &str)]) -> LosslessTree {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(GdscriptLanguage::kind_to_raw(K::SourceFile));
    for (kind, text) in parts {
        builder.token(GdscriptLanguage::kind_to_raw(*kind), text);
    }
    builder.finish_node();
    LosslessTree::from_green(builder.finish())
}

#[test]
fn directive_search_ignores_literals_suffixes_and_ordinary_comments() {
    for parts in [
        vec![(K::Comment, "# ordinary 中文 comment")],
        vec![(K::String, "'gdfmt:formatter-off'")],
        vec![
            (K::Identifier, "value"),
            (K::Whitespace, " "),
            (K::Comment, "# gdfmt:formatter-off"),
        ],
        vec![(K::Comment, "# prefix gdfmt:formatter-off")],
    ] {
        let tree = tree(&parts);
        assert!(directives::comments(&tree).is_empty());
        assert_eq!(directives::formatter_regions(&tree), Ok(vec![]));
    }
}

#[test]
fn directive_search_retains_bom_crlf_and_unknown_directive_behavior() {
    let tree = tree(&[
        (K::Bom, "\u{feff}"),
        (K::Comment, "#\tgdfmt:\tformatter-off"),
        (K::PhysicalNewline, "\r\n"),
        (K::Identifier, "unchanged"),
        (K::PhysicalNewline, "\r\n"),
        (K::Comment, "# gdfmt:formatter-on"),
        (K::PhysicalNewline, "\r\n"),
        (K::Comment, "# gdfmt:unknown"),
    ]);
    let comments = directives::comments(&tree);
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[2].1, "unknown");
    let regions = directives::formatter_regions(&tree).unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].slice(&tree.text()).unwrap(),
        "#\tgdfmt:\tformatter-off\r\nunchanged\r\n# gdfmt:formatter-on\r\n"
    );
}

#[test]
fn malformed_directive_pairs_still_report_the_original_marker_spans() {
    for body in ["formatter-off", "formatter-on"] {
        let marker = format!("# gdfmt:{body}");
        let tree = tree(&[(K::Comment, &marker)]);
        let comments = directives::comments(&tree);
        assert_eq!(
            directives::formatter_regions(&tree),
            Err(vec![comments[0].0])
        );
    }
    let tree = tree(&[
        (K::Comment, "# gdfmt:formatter-off"),
        (K::PhysicalNewline, "\n"),
        (K::Comment, "# gdfmt:formatter-off"),
        (K::PhysicalNewline, "\n"),
        (K::Comment, "# gdfmt:formatter-on"),
    ]);
    let comments = directives::comments(&tree);
    assert_eq!(
        directives::formatter_regions(&tree),
        Err(vec![comments[1].0])
    );
}
