use gdfmt_text::{LineIndex, Position, Span, TextEdit, TextError, apply_edits};

#[test]
fn byte_and_utf16_positions_differ_for_unicode() {
    let index = LineIndex::new("中😀\r\nvar x\n").unwrap();
    assert_eq!(
        index.byte_position(7).unwrap(),
        Position { line: 0, column: 7 }
    );
    assert_eq!(
        index.utf16_position(7).unwrap(),
        Position { line: 0, column: 3 }
    );
    assert_eq!(
        index.utf16_position(9).unwrap(),
        Position { line: 1, column: 0 }
    );
    assert_eq!(
        index.byte_position(15).unwrap(),
        Position { line: 2, column: 0 }
    );
    assert_eq!(index.byte_position(1), Err(TextError::InvalidBoundary));
    assert_eq!(index.byte_position(16), Err(TextError::InvalidBoundary));
    assert_eq!(
        LineIndex::new("").unwrap().byte_position(0).unwrap(),
        Position { line: 0, column: 0 }
    );
}

#[test]
fn edits_are_checked_then_applied_in_source_order() {
    let edits = [
        TextEdit {
            span: Span::new(4, 5).unwrap(),
            replacement: "y".into(),
        },
        TextEdit {
            span: Span::new(0, 3).unwrap(),
            replacement: "const".into(),
        },
    ];
    assert_eq!(apply_edits("var x", &edits).unwrap(), "const y");
    let overlap = [edits[0].clone(), edits[0].clone()];
    assert_eq!(
        apply_edits("var x", &overlap),
        Err(TextError::OverlappingEdits)
    );
    let bad = TextEdit {
        span: Span::new(1, 2).unwrap(),
        replacement: "".into(),
    };
    assert_eq!(apply_edits("中", &[bad]), Err(TextError::InvalidBoundary));
    assert!(Span::new(2, 1).is_err());
}
