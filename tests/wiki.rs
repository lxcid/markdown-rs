//! Tests for Obsidian-style wiki links and embeds.
//!
//! The grammar and node shapes are this fork's own (documented in
//! `src/construct/wiki.rs` and `src/mdast.rs`); there is no upstream
//! `remark` oracle, so these tests pin the chosen behavior.

use markdown::{
    mdast::{Node, Paragraph, Root, Text, WikiEmbed, WikiLink},
    message,
    unist::Position,
    Constructs, ParseOptions,
};
use pretty_assertions::assert_eq;

fn wiki_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            wiki_link: true,
            wiki_embed: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    }
}

#[test]
fn wiki_link_basic() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("[[Page]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiLink(WikiLink {
                    target: "Page".into(),
                    fragment: None,
                    alias: None,
                    position: Some(Position::new(1, 1, 0, 1, 9, 8))
                })],
                position: Some(Position::new(1, 1, 0, 1, 9, 8))
            })],
            position: Some(Position::new(1, 1, 0, 1, 9, 8))
        }),
        "should parse a basic wiki link"
    );
    Ok(())
}

#[test]
fn wiki_link_fragment() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("[[Page#Heading]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiLink(WikiLink {
                    target: "Page".into(),
                    fragment: Some("Heading".into()),
                    alias: None,
                    position: Some(Position::new(1, 1, 0, 1, 17, 16))
                })],
                position: Some(Position::new(1, 1, 0, 1, 17, 16))
            })],
            position: Some(Position::new(1, 1, 0, 1, 17, 16))
        }),
        "should parse a wiki link with a fragment"
    );
    Ok(())
}

#[test]
fn wiki_link_alias() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("[[Page|Alias]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiLink(WikiLink {
                    target: "Page".into(),
                    fragment: None,
                    alias: Some("Alias".into()),
                    position: Some(Position::new(1, 1, 0, 1, 15, 14))
                })],
                position: Some(Position::new(1, 1, 0, 1, 15, 14))
            })],
            position: Some(Position::new(1, 1, 0, 1, 15, 14))
        }),
        "should parse a wiki link with an alias"
    );
    Ok(())
}

#[test]
fn wiki_embed_basic() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("![[image.png]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiEmbed(WikiEmbed {
                    target: "image.png".into(),
                    fragment: None,
                    alias: None,
                    position: Some(Position::new(1, 1, 0, 1, 15, 14))
                })],
                position: Some(Position::new(1, 1, 0, 1, 15, 14))
            })],
            position: Some(Position::new(1, 1, 0, 1, 15, 14))
        }),
        "should parse a basic wiki embed"
    );
    Ok(())
}

#[test]
fn wiki_embed_fragment() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("![[data.csv#view]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiEmbed(WikiEmbed {
                    target: "data.csv".into(),
                    fragment: Some("view".into()),
                    alias: None,
                    position: Some(Position::new(1, 1, 0, 1, 19, 18))
                })],
                position: Some(Position::new(1, 1, 0, 1, 19, 18))
            })],
            position: Some(Position::new(1, 1, 0, 1, 19, 18))
        }),
        "should parse a wiki embed with a fragment"
    );
    Ok(())
}

#[test]
fn wiki_proving_case() -> Result<(), message::Message> {
    // Definition of done proving case.
    let tree = markdown::to_mdast("See [[Page|Alias]] and ![[image.png]].", &wiki_options())?;
    if let Node::Root(root) = &tree {
        if let Node::Paragraph(paragraph) = &root.children[0] {
            let kinds: Vec<&str> = paragraph
                .children
                .iter()
                .map(|child| match child {
                    Node::Text(_) => "text",
                    Node::WikiLink(_) => "wikiLink",
                    Node::WikiEmbed(_) => "wikiEmbed",
                    _ => "other",
                })
                .collect();
            assert_eq!(
                kinds,
                vec!["text", "wikiLink", "text", "wikiEmbed", "text"],
                "should resolve mixed wiki link + embed inline"
            );
            assert_eq!(
                paragraph.children[1],
                Node::WikiLink(WikiLink {
                    target: "Page".into(),
                    fragment: None,
                    alias: Some("Alias".into()),
                    position: Some(Position::new(1, 5, 4, 1, 19, 18))
                }),
                "wiki link should have correct fields + position"
            );
            assert_eq!(
                paragraph.children[3],
                Node::WikiEmbed(WikiEmbed {
                    target: "image.png".into(),
                    fragment: None,
                    alias: None,
                    position: Some(Position::new(1, 24, 23, 1, 38, 37))
                }),
                "wiki embed should have correct fields + position"
            );
            return Ok(());
        }
    }
    panic!("expected root > paragraph, got {:?}", tree);
}

#[test]
fn wiki_disabled_by_default() -> Result<(), message::Message> {
    // With default constructs, `[[Page]]` is plain text (links may still try to
    // form, but there is no wiki node).
    let tree = markdown::to_mdast("[[Page]]", &ParseOptions::default())?;
    let has_wiki = format!("{:?}", tree).contains("WikiLink");
    assert!(!has_wiki, "should not parse wiki links when disabled");
    Ok(())
}

#[test]
fn wiki_incomplete_is_text() -> Result<(), message::Message> {
    // A `[[` without a closing `]]` is not a wiki link.
    let tree = markdown::to_mdast("a [[unclosed", &wiki_options())?;
    let has_wiki = format!("{:?}", tree).contains("WikiLink");
    assert!(!has_wiki, "should not parse an unterminated wiki link");
    if let Node::Root(root) = &tree {
        if let Node::Paragraph(paragraph) = &root.children[0] {
            assert_eq!(
                paragraph.children,
                vec![Node::Text(Text {
                    value: "a [[unclosed".into(),
                    position: Some(Position::new(1, 1, 0, 1, 13, 12))
                })],
                "an unterminated wiki link should be plain text"
            );
        }
    }
    Ok(())
}
