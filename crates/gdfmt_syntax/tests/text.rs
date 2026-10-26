use gdfmt_syntax::{GdscriptLanguage, LosslessTree, SyntaxKind as K};
use rowan::{GreenNodeBuilder, Language};

#[test]
fn nested_green_text_preserves_every_byte_including_trivia_and_virtual_tokens() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(GdscriptLanguage::kind_to_raw(K::SourceFile));
    builder.token(GdscriptLanguage::kind_to_raw(K::Bom), "\u{feff}");
    builder.token(GdscriptLanguage::kind_to_raw(K::Comment), "# 中文 🦀");
    builder.token(GdscriptLanguage::kind_to_raw(K::PhysicalNewline), "\r\n");
    builder.start_node(GdscriptLanguage::kind_to_raw(K::VariableDecl));
    builder.token(GdscriptLanguage::kind_to_raw(K::Keyword), "var");
    builder.token(GdscriptLanguage::kind_to_raw(K::Whitespace), " ");
    builder.token(GdscriptLanguage::kind_to_raw(K::Identifier), "value");
    builder.token(GdscriptLanguage::kind_to_raw(K::Equal), "=");
    builder.start_node(GdscriptLanguage::kind_to_raw(K::LiteralExpr));
    builder.token(GdscriptLanguage::kind_to_raw(K::String), "'''a\r\n\tb'''");
    builder.finish_node();
    builder.token(GdscriptLanguage::kind_to_raw(K::Newline), "");
    builder.token(GdscriptLanguage::kind_to_raw(K::PhysicalNewline), "\r\n");
    builder.finish_node();
    builder.token(GdscriptLanguage::kind_to_raw(K::Eof), "");
    builder.finish_node();
    let tree = LosslessTree::from_green(builder.finish());
    let expected = "\u{feff}# 中文 🦀\r\nvar value='''a\r\n\tb'''\r\n";
    assert_eq!(tree.text(), expected);
    assert_eq!(tree.root().text(), expected);
    assert_eq!(tree.clone().text(), expected);
}

#[test]
fn empty_green_nodes_do_not_interrupt_text_order() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(GdscriptLanguage::kind_to_raw(K::SourceFile));
    builder.start_node(GdscriptLanguage::kind_to_raw(K::ErrorNode));
    builder.finish_node();
    builder.token(GdscriptLanguage::kind_to_raw(K::Error), "?");
    builder.start_node(GdscriptLanguage::kind_to_raw(K::ErrorNode));
    builder.finish_node();
    builder.finish_node();
    assert_eq!(LosslessTree::from_green(builder.finish()).text(), "?");

    let mut builder = GreenNodeBuilder::new();
    builder.start_node(GdscriptLanguage::kind_to_raw(K::SourceFile));
    builder.finish_node();
    assert_eq!(LosslessTree::from_green(builder.finish()).text(), "");
}
