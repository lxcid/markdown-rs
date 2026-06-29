//! Obsidian-style wiki links and embeds occur in the [text][] content type.
//!
//! ## Grammar
//!
//! Wiki links and embeds form with the following BNF
//! (<small>see [construct][crate::construct] for character groups</small>):
//!
//! ```bnf
//! wiki_link  ::= '[' '[' target [fragment] [alias] ']' ']'
//! wiki_embed ::= '!' '[' '[' target [fragment] [alias] ']' ']'
//!
//! target   ::= 1*(line - '#' - '|' - ']')
//! fragment ::= '#' *(line - '|' - ']')
//! alias    ::= '|' *(line - ']')
//! ```
//!
//! Both are single-line constructs (they do not span line endings).
//! The chosen grammar matches common Obsidian behavior:
//!
//! * `[[Page]]` — a link to `Page`.
//! * `[[Page#Heading]]` — a link to a heading within `Page`.
//! * `[[Page|Alias]]` — a link displayed as `Alias`.
//! * `![[image.png]]` — an embed of `image.png`.
//! * `![[data.csv#view]]` — an embed with a fragment.
//!
//! As wiki syntax lives in the text content type, it does not trigger inside
//! code (text), code (fenced), autolinks, or other raw spans, because those are
//! tokenized first.
//!
//! ## Tokens
//!
//! * [`WikiLink`][crate::event::Name::WikiLink]
//! * [`WikiEmbed`][crate::event::Name::WikiEmbed]
//! * [`WikiMarker`][crate::event::Name::WikiMarker]
//! * [`WikiTarget`][crate::event::Name::WikiTarget]
//! * [`WikiFragment`][crate::event::Name::WikiFragment]
//! * [`WikiFragmentMarker`][crate::event::Name::WikiFragmentMarker]
//! * [`WikiAlias`][crate::event::Name::WikiAlias]
//! * [`WikiAliasMarker`][crate::event::Name::WikiAliasMarker]
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of a wiki link.
///
/// ```markdown
/// > | a [[b]] c
///       ^
/// ```
pub fn link_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.wiki_link
        && tokenizer.current == Some(b'[')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 1) == Some(&b'[')
    {
        tokenizer.tokenize_state.token_1 = Name::WikiLink;
        tokenizer.tokenize_state.size = 2;
        tokenizer.enter(Name::WikiLink);
        tokenizer.enter(Name::WikiMarker);
        State::Retry(StateName::WikiBefore)
    } else {
        State::Nok
    }
}

/// Start of a wiki embed.
///
/// ```markdown
/// > | a ![[b]] c
///       ^
/// ```
pub fn embed_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.wiki_embed
        && tokenizer.current == Some(b'!')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 1) == Some(&b'[')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 2) == Some(&b'[')
    {
        tokenizer.tokenize_state.token_1 = Name::WikiEmbed;
        tokenizer.tokenize_state.size = 2;
        tokenizer.enter(Name::WikiEmbed);
        tokenizer.enter(Name::WikiMarker);
        tokenizer.consume();
        State::Next(StateName::WikiBefore)
    } else {
        State::Nok
    }
}

/// In the opening marker, consuming `[` characters.
///
/// ```markdown
/// > | a [[b]] c
///        ^
/// ```
pub fn before(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size > 0 && tokenizer.current == Some(b'[') {
        tokenizer.tokenize_state.size -= 1;
        tokenizer.consume();
        State::Next(StateName::WikiBefore)
    } else if tokenizer.tokenize_state.size == 0 {
        tokenizer.exit(Name::WikiMarker);
        tokenizer.enter(Name::WikiTarget);
        State::Retry(StateName::WikiTarget)
    } else {
        nok(tokenizer)
    }
}

/// In the target.
///
/// ```markdown
/// > | a [[b#c|d]] e
///        ^
/// ```
pub fn target(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') | Some(b'[') => nok(tokenizer),
        Some(b']') => {
            tokenizer.exit(Name::WikiTarget);
            State::Retry(StateName::WikiClose)
        }
        Some(b'#') => {
            tokenizer.exit(Name::WikiTarget);
            tokenizer.enter(Name::WikiFragmentMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiFragmentMarker);
            tokenizer.enter(Name::WikiFragment);
            State::Next(StateName::WikiFragment)
        }
        Some(b'|') => {
            tokenizer.exit(Name::WikiTarget);
            tokenizer.enter(Name::WikiAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiAliasMarker);
            tokenizer.enter(Name::WikiAlias);
            State::Next(StateName::WikiAlias)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::WikiTarget)
        }
    }
}

/// In the fragment (after `#`).
///
/// ```markdown
/// > | a [[b#c|d]] e
///          ^
/// ```
pub fn fragment(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') | Some(b'[') => nok(tokenizer),
        Some(b']') => {
            tokenizer.exit(Name::WikiFragment);
            State::Retry(StateName::WikiClose)
        }
        Some(b'|') => {
            tokenizer.exit(Name::WikiFragment);
            tokenizer.enter(Name::WikiAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiAliasMarker);
            tokenizer.enter(Name::WikiAlias);
            State::Next(StateName::WikiAlias)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::WikiFragment)
        }
    }
}

/// In the alias (after `|`).
///
/// ```markdown
/// > | a [[b#c|d]] e
///            ^
/// ```
pub fn alias(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') | Some(b'[') => nok(tokenizer),
        Some(b']') => {
            tokenizer.exit(Name::WikiAlias);
            State::Retry(StateName::WikiClose)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::WikiAlias)
        }
    }
}

/// At the closing `]]`.
///
/// ```markdown
/// > | a [[b]] c
///          ^
/// ```
pub fn close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b']')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 1) == Some(&b']')
    {
        tokenizer.enter(Name::WikiMarker);
        tokenizer.consume();
        State::Next(StateName::WikiCloseAfter)
    } else {
        nok(tokenizer)
    }
}

/// After the first closing `]`, at the second.
///
/// ```markdown
/// > | a [[b]] c
///           ^
/// ```
pub fn close_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.consume();
    tokenizer.exit(Name::WikiMarker);
    let name = tokenizer.tokenize_state.token_1.clone();
    tokenizer.exit(name);
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.size = 0;
    State::Ok
}

/// Reset state and signal failure.
fn nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.size = 0;
    State::Nok
}
