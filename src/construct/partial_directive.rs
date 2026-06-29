//! Shared sub-parsers for directives: name, label, and attributes.
//!
//! These factories are used by [directive (text)][crate::construct::directive_text],
//! [directive (leaf)][crate::construct::directive_leaf], and
//! [directive (container)][crate::construct::directive_container].
//!
//! They are ported from `micromark-extension-directive`’s `factory-name`,
//! `factory-label`, and `factory-attributes`.
//!
//! ## Grammar
//!
//! ```bnf
//! name  ::= name_start *name_cont
//! label ::= '[' *(text - ']' | '\\' any | '[' label ']') ']'
//! attributes ::= '{' *(whitespace (id | class | attribute)) whitespace '}'
//! id        ::= '#' 1*value_char
//! class     ::= '.' 1*value_char
//! attribute ::= attr_name ['=' (quoted | unquoted)]
//! ```
//!
//! * A directive `name` may not start with, or be, Unicode punctuation or
//!   whitespace; `-` and `_` may occur inside but not at the end.
//! * Attribute values are taken as raw slices (no HTML entity decoding yet).

use crate::event::{Content, Link, Name};
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::char::{kind_after_index, Kind as CharacterKind};

/// At the start of a directive name.
///
/// ```markdown
/// > | :name
///      ^
/// ```
pub fn name_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current.is_some()
        && kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index)
            == CharacterKind::Other
    {
        tokenizer.enter(Name::DirectiveName);
        tokenizer.consume();
        State::Next(StateName::DirectiveNameInside)
    } else {
        State::Nok
    }
}

/// In a directive name.
///
/// ```markdown
/// > | :name
///       ^
/// ```
pub fn name_inside(tokenizer: &mut Tokenizer) -> State {
    // Continuation byte of a multi-byte character.
    if matches!(tokenizer.current, Some(0x80..=0xBF)) {
        tokenizer.consume();
        return State::Next(StateName::DirectiveNameInside);
    }

    let kind = kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index);

    if kind == CharacterKind::Other || matches!(tokenizer.current, Some(b'-' | b'_')) {
        tokenizer.consume();
        State::Next(StateName::DirectiveNameInside)
    } else if matches!(tokenizer.previous, Some(b'-' | b'_')) {
        // A name may not end with `-` or `_`.
        State::Nok
    } else {
        tokenizer.exit(Name::DirectiveName);
        State::Ok
    }
}

/// At the start of a directive label (`[`).
///
/// ```markdown
/// > | :name[label]
///          ^
/// ```
pub fn label_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'[') {
        tokenizer.tokenize_state.size_c = 0;
        tokenizer.enter(Name::DirectiveLabel);
        tokenizer.enter(Name::DirectiveLabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::DirectiveLabelMarker);
        tokenizer.enter_link(
            Name::DirectiveLabelString,
            Link {
                previous: None,
                next: None,
                content: Content::Text,
            },
        );
        State::Next(StateName::DirectiveLabelInside)
    } else {
        State::Nok
    }
}

/// In a directive label.
///
/// ```markdown
/// > | :name[label]
///           ^
/// ```
pub fn label_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.tokenize_state.size_c = 0;
            State::Nok
        }
        Some(b'\\') => {
            tokenizer.consume();
            State::Next(StateName::DirectiveLabelEscape)
        }
        Some(b'[') => {
            tokenizer.tokenize_state.size_c += 1;
            tokenizer.consume();
            State::Next(StateName::DirectiveLabelInside)
        }
        Some(b']') => {
            if tokenizer.tokenize_state.size_c == 0 {
                tokenizer.exit(Name::DirectiveLabelString);
                tokenizer.enter(Name::DirectiveLabelMarker);
                tokenizer.consume();
                tokenizer.exit(Name::DirectiveLabelMarker);
                tokenizer.exit(Name::DirectiveLabel);
                State::Ok
            } else {
                tokenizer.tokenize_state.size_c -= 1;
                tokenizer.consume();
                State::Next(StateName::DirectiveLabelInside)
            }
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::DirectiveLabelInside)
        }
    }
}

/// After `\` in a directive label.
///
/// ```markdown
/// > | :name[a\]b]
///            ^
/// ```
pub fn label_escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.tokenize_state.size_c = 0;
            State::Nok
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::DirectiveLabelInside)
        }
    }
}

/// At the start of directive attributes (`{`).
///
/// ```markdown
/// > | :name{#id .cls key=value}
///          ^
/// ```
pub fn attributes_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'{') {
        tokenizer.enter(Name::DirectiveAttributes);
        tokenizer.enter(Name::DirectiveAttributesMarker);
        tokenizer.consume();
        tokenizer.exit(Name::DirectiveAttributesMarker);
        State::Next(StateName::DirectiveAttributesBetween)
    } else {
        State::Nok
    }
}

/// Between directive attributes.
///
/// ```markdown
/// > | :name{#id .cls key=value}
///           ^
/// ```
pub fn attributes_between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Whitespace separates attributes (kept loose, inside `DirectiveAttributes`).
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::DirectiveAttributesBetween)
        }
        Some(b'}') => {
            tokenizer.enter(Name::DirectiveAttributesMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DirectiveAttributesMarker);
            tokenizer.exit(Name::DirectiveAttributes);
            State::Ok
        }
        Some(b'#') => {
            tokenizer.enter(Name::DirectiveAttribute);
            tokenizer.enter(Name::DirectiveAttributeIdMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DirectiveAttributeIdMarker);
            tokenizer.tokenize_state.token_4 = Name::DirectiveAttributeId;
            tokenizer.enter(Name::DirectiveAttributeId);
            State::Next(StateName::DirectiveAttributeShortcutValue)
        }
        Some(b'.') => {
            tokenizer.enter(Name::DirectiveAttribute);
            tokenizer.enter(Name::DirectiveAttributeClassMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DirectiveAttributeClassMarker);
            tokenizer.tokenize_state.token_4 = Name::DirectiveAttributeClass;
            tokenizer.enter(Name::DirectiveAttributeClass);
            State::Next(StateName::DirectiveAttributeShortcutValue)
        }
        None | Some(b'\n') => State::Nok,
        Some(_) => {
            if is_attribute_name_start(tokenizer) {
                tokenizer.enter(Name::DirectiveAttribute);
                tokenizer.enter(Name::DirectiveAttributeName);
                tokenizer.consume();
                State::Next(StateName::DirectiveAttributeNameInside)
            } else {
                State::Nok
            }
        }
    }
}

/// In a `#id` or `.class` shortcut value.
///
/// ```markdown
/// > | :name{#id .cls}
///            ^^
/// ```
pub fn attribute_shortcut_value(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Nok,
        Some(b'#' | b'.' | b'}' | b'\t' | b' ') => {
            let name = tokenizer.tokenize_state.token_4.clone();
            tokenizer.exit(name);
            tokenizer.tokenize_state.token_4 = Name::Data;
            tokenizer.exit(Name::DirectiveAttribute);
            State::Retry(StateName::DirectiveAttributesBetween)
        }
        Some(b'"' | b'\'' | b'<' | b'=' | b'>' | b'`') => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::DirectiveAttributeShortcutValue)
        }
    }
}

/// In an attribute name.
///
/// ```markdown
/// > | :name{key=value}
///           ^^^
/// ```
pub fn attribute_name_inside(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(0x80..=0xBF)) {
        tokenizer.consume();
        return State::Next(StateName::DirectiveAttributeNameInside);
    }

    let kind = kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index);

    if kind == CharacterKind::Other || matches!(tokenizer.current, Some(b'-' | b'.' | b':' | b'_')) {
        tokenizer.consume();
        State::Next(StateName::DirectiveAttributeNameInside)
    } else {
        tokenizer.exit(Name::DirectiveAttributeName);
        State::Retry(StateName::DirectiveAttributeNameAfter)
    }
}

/// After an attribute name.
///
/// ```markdown
/// > | :name{key=value}
///              ^
/// ```
pub fn attribute_name_after(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'=') {
        tokenizer.enter(Name::DirectiveAttributeInitializerMarker);
        tokenizer.consume();
        tokenizer.exit(Name::DirectiveAttributeInitializerMarker);
        State::Next(StateName::DirectiveAttributeValueBefore)
    } else {
        // Bare attribute (value `""`).
        tokenizer.exit(Name::DirectiveAttribute);
        State::Retry(StateName::DirectiveAttributesBetween)
    }
}

/// Before an attribute value.
///
/// ```markdown
/// > | :name{key=value}
///               ^
/// ```
pub fn attribute_value_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'"' | b'\'') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            tokenizer.enter(Name::DirectiveAttributeValueMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DirectiveAttributeValueMarker);
            tokenizer.enter(Name::DirectiveAttributeValue);
            State::Next(StateName::DirectiveAttributeValueQuoted)
        }
        None | Some(b'\n' | b'\t' | b' ' | b'}' | b'<' | b'=' | b'>' | b'`') => State::Nok,
        Some(_) => {
            tokenizer.enter(Name::DirectiveAttributeValue);
            State::Retry(StateName::DirectiveAttributeValueUnquoted)
        }
    }
}

/// In a quoted attribute value.
///
/// ```markdown
/// > | :name{key="value"}
///                ^^^^^
/// ```
pub fn attribute_value_quoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.tokenize_state.marker = 0;
            State::Nok
        }
        Some(byte) if byte == tokenizer.tokenize_state.marker => {
            tokenizer.exit(Name::DirectiveAttributeValue);
            tokenizer.tokenize_state.marker = 0;
            tokenizer.enter(Name::DirectiveAttributeValueMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DirectiveAttributeValueMarker);
            tokenizer.exit(Name::DirectiveAttribute);
            State::Next(StateName::DirectiveAttributesBetween)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::DirectiveAttributeValueQuoted)
        }
    }
}

/// In an unquoted attribute value.
///
/// ```markdown
/// > | :name{key=value}
///               ^^^^^
/// ```
pub fn attribute_value_unquoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n' | b'\t' | b' ' | b'}') => {
            tokenizer.exit(Name::DirectiveAttributeValue);
            tokenizer.exit(Name::DirectiveAttribute);
            State::Retry(StateName::DirectiveAttributesBetween)
        }
        Some(b'"' | b'\'' | b'<' | b'=' | b'>' | b'`') => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::DirectiveAttributeValueUnquoted)
        }
    }
}

/// Whether the current byte can start an attribute name.
fn is_attribute_name_start(tokenizer: &Tokenizer) -> bool {
    matches!(tokenizer.current, Some(b'-' | b'_'))
        || kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index)
            == CharacterKind::Other
}
