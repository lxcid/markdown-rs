//! Round-trip tests for this fork's wiki link / embed nodes.

use markdown::{
    mdast::{Node, WikiEmbed, WikiLink},
    to_mdast, Constructs, ParseOptions,
};
use mdast_util_to_markdown::to_markdown as to;
use pretty_assertions::assert_eq;

fn wiki_parse() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            wiki: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    }
}

/// Parse markdown, serialize the tree, and return the result.
fn roundtrip(src: &str) -> String {
    to(&to_mdast(src, &wiki_parse()).unwrap()).unwrap()
}

#[test]
fn wiki_link_serialize() {
    assert_eq!(
        to(&Node::WikiLink(WikiLink {
            target: "Page".into(),
            fragment: None,
            alias: None,
            position: None,
        }))
        .unwrap(),
        "[[Page]]\n",
        "should serialize a basic wiki link"
    );

    assert_eq!(
        to(&Node::WikiLink(WikiLink {
            target: "Page".into(),
            fragment: Some("H".into()),
            alias: Some("Alias".into()),
            position: None,
        }))
        .unwrap(),
        "[[Page#H|Alias]]\n",
        "should serialize target#fragment|alias in order"
    );

    assert_eq!(
        to(&Node::WikiLink(WikiLink {
            target: String::new(),
            fragment: Some("Heading".into()),
            alias: None,
            position: None,
        }))
        .unwrap(),
        "[[#Heading]]\n",
        "should serialize an empty-target same-note link"
    );
}

#[test]
fn wiki_embed_serialize() {
    assert_eq!(
        to(&Node::WikiEmbed(WikiEmbed {
            target: "clip.mp4".into(),
            fragment: Some("t=10".into()),
            alias: None,
            position: None,
        }))
        .unwrap(),
        "![[clip.mp4#t=10]]\n",
        "should serialize an embed with a leading `!`"
    );
}

/// Parsing then serializing returns the original source (plus the serializer's
/// trailing newline).
#[test]
fn wiki_roundtrip() {
    for src in [
        "[[Page]]",
        "[[Page#Heading]]",
        "[[Page|Alias]]",
        "[[Page#Heading|Alias]]",
        "[[#Heading]]",
        "[[#^block]]",
        "![[image.png]]",
        "![[note#^block]]",
        "![[clip.mp4#t=10|My Video]]",
    ] {
        assert_eq!(
            roundtrip(src),
            format!("{}\n", src),
            "should round-trip {:?}",
            src
        );
    }
}

/// `WikiLink` / `WikiEmbed` fields are public, so a tree can be built by hand
/// with arbitrary strings. A field that would reparse differently has no
/// escaped form, so the serializer must reject it rather than emit markdown
/// that changes meaning.
#[test]
fn wiki_rejects_unrepresentable_fields() {
    let cases = [
        // A `#` in the target would reparse as a fragment boundary.
        WikiLink {
            target: "Page#literal".into(),
            fragment: None,
            alias: None,
            position: None,
        },
        // A `|` in the target would reparse as an alias boundary.
        WikiLink {
            target: "Page|literal".into(),
            fragment: None,
            alias: None,
            position: None,
        },
        // A `]` in any field breaks the `[[ … ]]` framing.
        WikiLink {
            target: "Page".into(),
            fragment: None,
            alias: Some("alias]]tail".into()),
            position: None,
        },
        // A bracket in the fragment, likewise.
        WikiLink {
            target: "Page".into(),
            fragment: Some("a[b".into()),
            alias: None,
            position: None,
        },
        // A line ending: the construct is single-line.
        WikiLink {
            target: "Page".into(),
            fragment: Some("a\nb".into()),
            alias: None,
            position: None,
        },
        // A `|` in the fragment would open an alias.
        WikiLink {
            target: "Page".into(),
            fragment: Some("a|b".into()),
            alias: None,
            position: None,
        },
        // An empty fragment/alias reparses as plain text (would not round-trip).
        WikiLink {
            target: "Page".into(),
            fragment: Some(String::new()),
            alias: None,
            position: None,
        },
        WikiLink {
            target: "Page".into(),
            fragment: None,
            alias: Some(String::new()),
            position: None,
        },
        // No target and no fragment reparses as plain text.
        WikiLink {
            target: String::new(),
            fragment: None,
            alias: Some("Alias".into()),
            position: None,
        },
    ];

    for link in cases {
        assert!(
            to(&Node::WikiLink(link.clone())).is_err(),
            "should reject an unrepresentable wiki link: {:?}",
            link
        );
    }

    // Embeds share the same validation path.
    assert!(
        to(&Node::WikiEmbed(WikiEmbed {
            target: "a]]b".into(),
            fragment: None,
            alias: None,
            position: None,
        }))
        .is_err(),
        "should reject an unrepresentable wiki embed"
    );

    // A same-note link (empty target, non-empty fragment) is still valid.
    assert_eq!(
        to(&Node::WikiLink(WikiLink {
            target: String::new(),
            fragment: Some("Heading".into()),
            alias: None,
            position: None,
        }))
        .unwrap(),
        "[[#Heading]]\n",
        "an empty target with a fragment should still serialize"
    );
}
