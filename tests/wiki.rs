//! Tests for Obsidian-style wiki links and embeds.
//!
//! The grammar and node shapes are this fork's own (documented in
//! `src/construct/wiki.rs` and `src/mdast.rs`); there is no upstream
//! `remark` oracle, so these tests pin the chosen behavior:
//!
//! ```text
//! [[ target ['#' fragment] ['|' alias] ]]
//! ![[ ... ]]
//! ```

use markdown::{
    mdast::{Node, Paragraph, Root, Text, WikiEmbed, WikiLink},
    message,
    to_html_with_options,
    unist::Position,
    Constructs, Options, ParseOptions,
};
use pretty_assertions::assert_eq;

fn wiki_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            wiki: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    }
}

fn wiki_html_options() -> Options {
    Options {
        parse: wiki_options(),
        ..Options::default()
    }
}

/// Parse `src` and return the first paragraph's children (structural assertions
/// read better than matching on `format!("{:?}")`).
fn paragraph_children(src: &str) -> Result<Vec<Node>, message::Message> {
    match markdown::to_mdast(src, &wiki_options())? {
        Node::Root(root) => match root.children.into_iter().next() {
            Some(Node::Paragraph(paragraph)) => Ok(paragraph.children),
            other => panic!("expected a paragraph for {:?}, got {:?}", src, other),
        },
        other => panic!("expected a root for {:?}, got {:?}", src, other),
    }
}

/// Collect the variant names of wiki/link/image nodes anywhere in a tree.
fn node_kinds(node: &Node, out: &mut Vec<&'static str>) {
    match node {
        Node::WikiLink(_) => out.push("WikiLink"),
        Node::WikiEmbed(_) => out.push("WikiEmbed"),
        Node::Link(_) => out.push("Link"),
        Node::Image(_) => out.push("Image"),
        _ => {}
    }
    if let Some(children) = node.children() {
        for child in children {
            node_kinds(child, out);
        }
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
fn wiki_link_fragment_alias() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("[[Page#Heading|Alias]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiLink(WikiLink {
                    target: "Page".into(),
                    fragment: Some("Heading".into()),
                    alias: Some("Alias".into()),
                    position: Some(Position::new(1, 1, 0, 1, 23, 22))
                })],
                position: Some(Position::new(1, 1, 0, 1, 23, 22))
            })],
            position: Some(Position::new(1, 1, 0, 1, 23, 22))
        }),
        "should parse target#fragment|alias in order"
    );
    Ok(())
}

/// Only the first `#` opens the fragment and the first `|` opens the alias;
/// later occurrences are literal. There is no upstream oracle for this fork
/// grammar, and the serializer validation depends on it, so pin it.
#[test]
fn wiki_repeated_separators_are_literal() -> Result<(), message::Message> {
    // A second `#` is part of the fragment, not a new segment.
    assert_eq!(
        paragraph_children("[[a#b#c]]")?,
        vec![Node::WikiLink(WikiLink {
            target: "a".into(),
            fragment: Some("b#c".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 10, 9))
        })],
        "a later `#` should be literal in the fragment"
    );

    // A second `|` is part of the alias, not a new segment.
    assert_eq!(
        paragraph_children("[[a|b|c]]")?,
        vec![Node::WikiLink(WikiLink {
            target: "a".into(),
            fragment: None,
            alias: Some("b|c".into()),
            position: Some(Position::new(1, 1, 0, 1, 10, 9))
        })],
        "a later `|` should be literal in the alias"
    );

    // Embeds share the grammar.
    assert_eq!(
        paragraph_children("![[a#b#c]]")?,
        vec![Node::WikiEmbed(WikiEmbed {
            target: "a".into(),
            fragment: Some("b#c".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 11, 10))
        })],
        "a later `#` should be literal in an embed fragment"
    );
    assert_eq!(
        paragraph_children("![[a|b|c]]")?,
        vec![Node::WikiEmbed(WikiEmbed {
            target: "a".into(),
            fragment: None,
            alias: Some("b|c".into()),
            position: Some(Position::new(1, 1, 0, 1, 11, 10))
        })],
        "a later `|` should be literal in an embed alias"
    );
    Ok(())
}

/// Empty target is Obsidian's same-note link: `[[#Heading]]`, `[[#^block]]`.
#[test]
fn wiki_link_same_note() -> Result<(), message::Message> {
    let children = paragraph_children("[[#Heading]]")?;
    assert_eq!(
        children,
        vec![Node::WikiLink(WikiLink {
            target: String::new(),
            fragment: Some("Heading".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 13, 12))
        })],
        "should parse a same-note heading link with an empty target"
    );

    let block = paragraph_children("[[#^block]]")?;
    assert_eq!(
        block,
        vec![Node::WikiLink(WikiLink {
            target: String::new(),
            fragment: Some("^block".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 12, 11))
        })],
        "should parse a same-note block link"
    );
    Ok(())
}

/// `?` is an ordinary `target` character in this grammar, not a delimiter.
#[test]
fn wiki_question_mark_is_literal() -> Result<(), message::Message> {
    assert_eq!(
        paragraph_children("[[Page?x]]")?,
        vec![Node::WikiLink(WikiLink {
            target: "Page?x".into(),
            fragment: None,
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 11, 10))
        })],
        "`?` should be part of the target, not a query delimiter"
    );
    assert_eq!(
        to_html_with_options("[[Page?x]]", &wiki_html_options())?,
        "<p><a href=\"Page?x\">Page?x</a></p>",
        "`?` should be literal in the href"
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
fn wiki_embed_fragment_alias() -> Result<(), message::Message> {
    assert_eq!(
        markdown::to_mdast("![[clip.mp4#t=10|My Video]]", &wiki_options())?,
        Node::Root(Root {
            children: vec![Node::Paragraph(Paragraph {
                children: vec![Node::WikiEmbed(WikiEmbed {
                    target: "clip.mp4".into(),
                    fragment: Some("t=10".into()),
                    alias: Some("My Video".into()),
                    position: Some(Position::new(1, 1, 0, 1, 28, 27))
                })],
                position: Some(Position::new(1, 1, 0, 1, 28, 27))
            })],
            position: Some(Position::new(1, 1, 0, 1, 28, 27))
        }),
        "should parse an embed with fragment and alias"
    );
    Ok(())
}

/// Embeds support the same empty-target form (a same-note block transclusion).
#[test]
fn wiki_embed_same_note() -> Result<(), message::Message> {
    let with_target = paragraph_children("![[note#^block]]")?;
    assert_eq!(
        with_target,
        vec![Node::WikiEmbed(WikiEmbed {
            target: "note".into(),
            fragment: Some("^block".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 17, 16))
        })],
        "should parse an embed of a block within a note"
    );

    let same_note = paragraph_children("![[#^block]]")?;
    assert_eq!(
        same_note,
        vec![Node::WikiEmbed(WikiEmbed {
            target: String::new(),
            fragment: Some("^block".into()),
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 13, 12))
        })],
        "should parse a same-note block embed with an empty target"
    );
    Ok(())
}

/// An empty trailing segment (`#`/`|` immediately before `]]`) is not a wiki
/// construct: a bare `#`/`|` carries no heading or alias, so `[[Page#]]`,
/// `[[Page|]]`, and `[[Page#|Alias]]` stay plain text (and so round-trip).
#[test]
fn wiki_empty_trailing_segments_are_text() -> Result<(), message::Message> {
    for src in ["[[Page#]]", "[[Page|]]", "[[Page#|Alias]]"] {
        let children = paragraph_children(src)?;
        let mut kinds = vec![];
        for child in &children {
            node_kinds(child, &mut kinds);
        }
        assert!(
            !kinds.contains(&"WikiLink") && !kinds.contains(&"WikiEmbed"),
            "{:?} should not parse as a wiki node, got {:?}",
            src,
            children
        );
        let text: String = children.iter().map(Node::to_string).collect();
        assert_eq!(text, src, "{:?} should be its literal text", src);
    }
    Ok(())
}

/// The target or fragment must be non-empty; otherwise the spelling is plain
/// text (an alias alone does not count).
#[test]
fn wiki_truly_empty_is_text() -> Result<(), message::Message> {
    for src in ["[[]]", "[[#]]", "[[|x]]"] {
        let children = paragraph_children(src)?;
        let mut kinds = vec![];
        for child in &children {
            node_kinds(child, &mut kinds);
        }
        assert!(
            !kinds.contains(&"WikiLink") && !kinds.contains(&"WikiEmbed"),
            "{:?} should not parse as a wiki node, got {:?}",
            src,
            children
        );
        let text: String = children.iter().map(Node::to_string).collect();
        assert_eq!(text, src, "{:?} should be its literal text", src);
    }
    Ok(())
}

#[test]
fn wiki_incomplete_is_text() -> Result<(), message::Message> {
    // A `[[` without a closing `]]` is not a wiki link.
    let children = paragraph_children("a [[unclosed")?;
    assert_eq!(
        children,
        vec![Node::Text(Text {
            value: "a [[unclosed".into(),
            position: Some(Position::new(1, 1, 0, 1, 13, 12))
        })],
        "an unterminated wiki link should be plain text"
    );
    Ok(())
}

/// Wiki syntax is shielded inside a normal link or image label so that the
/// bracket boundaries resolve to the surrounding link/image.
#[test]
fn wiki_shielded_in_label() -> Result<(), message::Message> {
    let link = markdown::to_mdast("[a [[b]]](c)", &wiki_options())?;
    let mut link_kinds = vec![];
    node_kinds(&link, &mut link_kinds);
    assert_eq!(
        link_kinds,
        vec!["Link"],
        "the surrounding link should form, with no wiki node inside its label"
    );

    let image = markdown::to_mdast("![alt [[b]]](c)", &wiki_options())?;
    let mut image_kinds = vec![];
    node_kinds(&image, &mut image_kinds);
    assert_eq!(
        image_kinds,
        vec!["Image"],
        "the surrounding image should form, with no wiki node inside its label"
    );
    Ok(())
}

/// The shield is owned by the bracket-label machinery, so it suppresses wiki
/// syntax while *any* link/image label opener is active — even one that never
/// resolves to a link. This is deliberately broader than a formed label.
#[test]
fn wiki_shielded_under_open_label() -> Result<(), message::Message> {
    for src in ["[a [[b]]", "![a ![[b]]"] {
        let tree = markdown::to_mdast(src, &wiki_options())?;
        let mut kinds = vec![];
        node_kinds(&tree, &mut kinds);
        assert!(
            !kinds.contains(&"WikiLink") && !kinds.contains(&"WikiEmbed"),
            "{:?} should stay text while a label opener is unresolved, got {:?}",
            src,
            kinds
        );
    }
    Ok(())
}

/// Frontmatter content is not text-tokenized, so wiki syntax inside it is inert.
#[test]
fn wiki_shielded_in_frontmatter() -> Result<(), message::Message> {
    let options = ParseOptions {
        constructs: Constructs {
            wiki: true,
            frontmatter: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    };
    let tree = markdown::to_mdast("---\ntitle: [[Page]]\n---\n", &options)?;
    if let Node::Root(root) = &tree {
        assert!(
            matches!(root.children.first(), Some(Node::Yaml(_))),
            "frontmatter should parse as YAML, got {:?}",
            root.children.first()
        );
    }
    let mut kinds = vec![];
    node_kinds(&tree, &mut kinds);
    assert!(
        !kinds.contains(&"WikiLink"),
        "wiki syntax inside frontmatter should be inert, got {:?}",
        kinds
    );
    Ok(())
}

/// Wiki syntax lives in the text content type, so it does not trigger inside
/// constructs that are tokenized first and hold their content raw: inline code,
/// indented/fenced code, autolinks, and HTML.
#[test]
fn wiki_shielded_in_raw_constructs() -> Result<(), message::Message> {
    for src in [
        "`[[Page]]`",                     // inline code
        "    [[Page]]",                   // indented code
        "```\n[[Page]]\n```",             // fenced code
        "<https://example.com/[[Page]]>", // autolink
        "<div>[[Page]]</div>",            // HTML flow
    ] {
        let tree = markdown::to_mdast(src, &wiki_options())?;
        let mut kinds = vec![];
        node_kinds(&tree, &mut kinds);
        assert!(
            !kinds.contains(&"WikiLink"),
            "wiki syntax in {:?} should be inert, got {:?}",
            src,
            kinds
        );
    }
    Ok(())
}

/// `[[` opens a wiki link before a normal `[label](url)` can form, so the wiki
/// link wins and the trailing `(url)` stays literal text.
#[test]
fn wiki_precedence_over_link() -> Result<(), message::Message> {
    let children = paragraph_children("[[Page]](url)")?;
    let mut kinds = vec![];
    for child in &children {
        node_kinds(child, &mut kinds);
    }
    assert_eq!(kinds, vec!["WikiLink"], "the wiki link should win");
    assert_eq!(
        children[0],
        Node::WikiLink(WikiLink {
            target: "Page".into(),
            fragment: None,
            alias: None,
            position: Some(Position::new(1, 1, 0, 1, 9, 8))
        }),
        "the wiki link should cover only `[[Page]]`"
    );
    assert_eq!(
        to_html_with_options("[[Page]](url)", &wiki_html_options())?,
        "<p><a href=\"Page\">Page</a>(url)</p>",
        "the trailing `(url)` should render as literal text"
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
    let mut kinds = vec![];
    node_kinds(&tree, &mut kinds);
    assert!(
        !kinds.contains(&"WikiLink") && !kinds.contains(&"WikiEmbed"),
        "should not parse wiki links when disabled, got {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn wiki_html_link() -> Result<(), message::Message> {
    assert_eq!(
        to_html_with_options("[[Page]]", &wiki_html_options())?,
        "<p><a href=\"Page\">Page</a></p>",
        "should render a wiki link as an anchor"
    );
    assert_eq!(
        to_html_with_options("[[Page|Alias]]", &wiki_html_options())?,
        "<p><a href=\"Page\">Alias</a></p>",
        "should render the alias as link text"
    );
    Ok(())
}

#[test]
fn wiki_html_link_fragment() -> Result<(), message::Message> {
    assert_eq!(
        to_html_with_options("[[Page#H|Alias]]", &wiki_html_options())?,
        "<p><a href=\"Page#H\">Alias</a></p>",
        "should build the href from target#fragment"
    );
    Ok(())
}

/// Embeds render as `<a>` regardless of the target's extension: there is no
/// media classifier, so inlining a transclusion is left to a resolver reading
/// the mdast node. The fragment is folded into the href.
#[test]
fn wiki_html_embed() -> Result<(), message::Message> {
    assert_eq!(
        to_html_with_options("![[image.png]]", &wiki_html_options())?,
        "<p><a href=\"image.png\">image.png</a></p>",
        "an image-extension embed should render as a link"
    );
    assert_eq!(
        to_html_with_options("![[clip.mp4#t=10|My Video]]", &wiki_html_options())?,
        "<p><a href=\"clip.mp4#t=10\">My Video</a></p>",
        "a media-extension embed should render as a link, fragment in the href"
    );
    assert_eq!(
        to_html_with_options("![[Some Note]]", &wiki_html_options())?,
        "<p><a href=\"Some%20Note\">Some Note</a></p>",
        "a note embed should render as a link"
    );
    assert_eq!(
        to_html_with_options("![[notes/Daily#Today|Today]]", &wiki_html_options())?,
        "<p><a href=\"notes/Daily#Today\">Today</a></p>",
        "a path embed with fragment + alias should render as a link"
    );
    Ok(())
}

/// Empty-target links build a fragment-only href and fall back to the fragment
/// for the display text.
#[test]
fn wiki_html_same_note() -> Result<(), message::Message> {
    assert_eq!(
        to_html_with_options("[[#Heading]]", &wiki_html_options())?,
        "<p><a href=\"#Heading\">Heading</a></p>",
        "should render a same-note heading link"
    );
    Ok(())
}

/// `WikiLink`/`WikiEmbed` serialize with a `type` tag and skip `None` segments.
#[cfg(feature = "json")]
#[test]
fn wiki_serde_output() {
    let link = Node::WikiLink(WikiLink {
        target: "Page".into(),
        fragment: Some("H".into()),
        alias: None,
        position: None,
    });
    assert_eq!(
        serde_json::to_string(&link).unwrap(),
        r#"{"type":"wikiLink","target":"Page","fragment":"H"}"#,
        "should serialize a wiki link, skipping the absent alias/position"
    );

    let embed = Node::WikiEmbed(WikiEmbed {
        target: String::new(),
        fragment: Some("^block".into()),
        alias: None,
        position: None,
    });
    assert_eq!(
        serde_json::to_string(&embed).unwrap(),
        r#"{"type":"wikiEmbed","target":"","fragment":"^block"}"#,
        "should serialize an empty-target embed"
    );
}
