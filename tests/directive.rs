//! Tests for `remark-directive`-compatible directives.
//!
//! Expected shapes were cross-checked against `remark-parse` + `remark-gfm` +
//! `remark-directive` (see the spec captured during development).

use markdown::{
    mdast::{
        ContainerDirective, LeafDirective, Node, Paragraph, Root, Text, TextDirective,
    },
    message,
    unist::Position,
    Constructs, ParseOptions,
};
use pretty_assertions::assert_eq;

fn directive_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            directive: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    }
}

/// Map a tree to a compact `(type, name)` shape so structural proving cases stay
/// readable.
fn shape(node: &Node) -> String {
    let label = match node {
        Node::Root(_) => "root".into(),
        Node::Blockquote(_) => "blockquote".into(),
        Node::Paragraph(_) => "paragraph".into(),
        Node::Text(t) => format!("text({:?})", t.value),
        Node::List(_) => "list".into(),
        Node::ListItem(_) => "listItem".into(),
        Node::ContainerDirective(d) => format!("containerDirective({})", d.name),
        Node::LeafDirective(d) => format!("leafDirective({})", d.name),
        Node::TextDirective(d) => format!("textDirective({})", d.name),
        other => format!("{:?}", core::mem::discriminant(other)),
    };
    if let Some(children) = node.children() {
        let inner: Vec<String> = children.iter().map(shape).collect();
        if inner.is_empty() {
            label
        } else {
            format!("{}[{}]", label, inner.join(","))
        }
    } else {
        label
    }
}

#[test]
fn text_directive() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast(":text[label]{key=value}", &directive_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::TextDirective(TextDirective {
                    name: "text".into(),
                    attributes: vec![("key".into(), "value".into())],
                    children: vec![Node::Text(Text {
                        value: "label".into(),
                        position: Some(Position::new(1, 7, 6, 1, 12, 11))
                    })],
                    position: Some(Position::new(1, 1, 0, 1, 24, 23))
                })],
                position: Some(Position::new(1, 1, 0, 1, 24, 23))
            })],
            position: Some(Position::new(1, 1, 0, 1, 24, 23))
        }),
        "should parse a text directive with a label and attributes"
    );
    Ok(())
}

#[test]
fn leaf_directive() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("::video[label]{src=clip.mp4}", &directive_options())?,
        Node::Root(Root {
            children: vec![Node::LeafDirective(LeafDirective {
                name: "video".into(),
                attributes: vec![("src".into(), "clip.mp4".into())],
                children: vec![Node::Text(Text {
                    value: "label".into(),
                    position: Some(Position::new(1, 9, 8, 1, 14, 13))
                })],
                position: Some(Position::new(1, 1, 0, 1, 29, 28))
            })],
            position: Some(Position::new(1, 1, 0, 1, 29, 28))
        }),
        "should parse a leaf directive (as a flow node) with a label and attributes"
    );
    Ok(())
}

#[test]
fn container_directive_basic() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast(":::note\ncontent\n:::", &directive_options())?,
        Node::Root(Root {
            children: vec![Node::ContainerDirective(ContainerDirective {
                name: "note".into(),
                attributes: vec![],
                children: vec![Node::Paragraph(Paragraph {
                    children: vec![Node::Text(Text {
                        value: "content".into(),
                        position: Some(Position::new(2, 1, 8, 2, 8, 15))
                    })],
                    position: Some(Position::new(2, 1, 8, 2, 8, 15))
                })],
                position: Some(Position::new(1, 1, 0, 3, 4, 19))
            })],
            position: Some(Position::new(1, 1, 0, 3, 4, 19))
        }),
        "should parse an empty-attributes container directive with a paragraph body"
    );
    Ok(())
}

#[test]
fn container_directive_attributes() -> Result<(), message::Message> {
    let tree = markdown::to_mdast(":::note{title=\"Hello\"}\ncontent\n:::", &directive_options())?;
    if let Node::Root(root) = &tree {
        if let Node::ContainerDirective(directive) = &root.children[0] {
            assert_eq!(directive.name, "note");
            assert_eq!(
                directive.attributes,
                vec![("title".into(), "Hello".into())],
                "should parse a quoted attribute value"
            );
            return Ok(());
        }
    }
    panic!("expected container directive, got {:?}", tree);
}

#[test]
fn directive_attributes_id_class() -> Result<(), message::Message> {
    let tree = markdown::to_mdast(":x{#i .c1 .c2 k=v}", &directive_options())?;
    let text = format!("{:?}", tree);
    assert!(text.contains("textDirective") || text.contains("TextDirective"));
    if let Node::Root(root) = &tree {
        if let Node::Paragraph(p) = &root.children[0] {
            if let Node::TextDirective(d) = &p.children[0] {
                assert_eq!(
                    d.attributes,
                    vec![
                        ("id".into(), "i".into()),
                        ("class".into(), "c1 c2".into()),
                        ("k".into(), "v".into()),
                    ],
                    "id last-wins, classes space-joined, key=value preserved"
                );
                return Ok(());
            }
        }
    }
    panic!("expected text directive, got {:?}", tree);
}

#[test]
fn proving_case_list_in_container() -> Result<(), message::Message> {
    // :::note / - a / ::: / after
    let tree = markdown::to_mdast(":::note\n- a\n:::\nafter", &directive_options())?;
    assert_eq!(
        shape(&tree),
        "root[containerDirective(note)[list[listItem[paragraph[text(\"a\")]]]],paragraph[text(\"after\")]]",
        "list nests inside the container; `after` is a sibling paragraph"
    );
    Ok(())
}

#[test]
fn proving_case_directive_interrupts_list() -> Result<(), message::Message> {
    // - a / :::note / x / ::: / after
    let tree = markdown::to_mdast("- a\n:::note\nx\n:::\nafter", &directive_options())?;
    assert_eq!(
        shape(&tree),
        "root[list[listItem[paragraph[text(\"a\")]]],containerDirective(note)[paragraph[text(\"x\")]],paragraph[text(\"after\")]]",
        "the directive interrupts the list and becomes a top-level sibling"
    );
    Ok(())
}

#[test]
fn proving_case_nested_containers() -> Result<(), message::Message> {
    // :::outer / :::inner / x / ::: / :::
    let tree = markdown::to_mdast(":::outer\n:::inner\nx\n:::\n:::", &directive_options())?;
    assert_eq!(
        shape(&tree),
        "root[containerDirective(outer)[containerDirective(inner)[paragraph[text(\"x\")]]]]",
        "containers nest, innermost closes first"
    );
    Ok(())
}

#[test]
fn container_directive_eof_auto_close() -> Result<(), message::Message> {
    let tree = markdown::to_mdast(":::note\ncontent without close", &directive_options())?;
    if let Node::Root(root) = &tree {
        if let Node::ContainerDirective(directive) = &root.children[0] {
            assert_eq!(directive.name, "note");
            assert_eq!(
                directive.position,
                Some(Position::new(1, 1, 0, 2, 22, 29)),
                "EOF should auto-close the container"
            );
            return Ok(());
        }
    }
    panic!("expected container directive, got {:?}", tree);
}

#[test]
fn directives_disabled_by_default() -> Result<(), message::Message> {
    let tree = markdown::to_mdast(":::note\ncontent\n:::", &ParseOptions::default())?;
    let text = format!("{:?}", tree);
    assert!(
        !text.contains("Directive"),
        "should not parse directives when disabled"
    );
    Ok(())
}

#[test]
fn directive_after_blockquote_without_blank() -> Result<(), message::Message> {
    // A leaf directive after a block quote line, no blank line between.
    let tree = markdown::to_mdast("> a\n::note", &directive_options())?;
    assert_eq!(
        shape(&tree),
        "root[blockquote[paragraph[text(\"a\")]],leafDirective(note)]",
        "leaf directive after a block quote becomes a sibling"
    );
    Ok(())
}

#[test]
fn directive_shielded_by_code_fence() -> Result<(), message::Message> {
    let tree = markdown::to_mdast("```\n:::note\n:::\n```", &directive_options())?;
    let text = format!("{:?}", tree);
    assert!(
        !text.contains("Directive"),
        "directives inside a code fence are not parsed"
    );
    Ok(())
}

#[test]
fn text_directive_negatives() -> Result<(), message::Message> {
    // foo:bar -> text("foo") + textDirective(bar)
    assert_eq!(shape(&markdown::to_mdast("foo:bar baz", &directive_options())?),
        "root[paragraph[text(\"foo\"),textDirective(bar),text(\" baz\")]]");
    // a ::b -> plain text (previous-colon guard)
    let t = markdown::to_mdast("a ::b inline", &directive_options())?;
    assert!(!format!("{:?}", t).contains("Directive"), "a ::b is plain text");
    // escaped colon
    let t2 = markdown::to_mdast("\\:notdirective", &directive_options())?;
    assert!(!format!("{:?}", t2).contains("Directive"), "escaped colon is text");
    Ok(())
}
