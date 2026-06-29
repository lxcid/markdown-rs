//! Obsidian-style wiki links and embeds occur in the [text][] content type.
//!
//! ## Grammar
//!
//! Wiki links and embeds form with the following BNF
//! (<small>see [construct][crate::construct] for character groups</small>):
//!
//! ```bnf
//! wiki_link  ::= '[' '[' body ']' ']'
//! wiki_embed ::= '!' '[' '[' body ']' ']'
//!
//! body     ::= target [fragment] [alias]
//!            | fragment [alias]
//!
//! target   ::= 1*(line - '#' - '|' - '[' - ']')
//! fragment ::= '#' 1*(line - '|' - '[' - ']')
//! alias    ::= '|' 1*(line - '[' - ']')
//! ```
//!
//! The two `body` forms encode one invariant: **the target or the fragment must
//! be non-empty.** The second form (empty target) is Obsidian's same-note link,
//! e.g. `[[#Heading]]` / `![[note#^block]]`. The degenerate spellings `[[]]`,
//! `[[#]]`, and `[[|x]]` (an alias alone) are plain text.
//!
//! Both are single-line constructs (they do not span line endings). A `target`
//! comes first, then an optional `#fragment`, and finally an optional `|alias`;
//! the first `#` opens the fragment and the first `|` opens the alias, later
//! occurrences being literal. Each present segment must be non-empty: a bare
//! trailing `#` or `|` (`[[Page#]]`, `[[Page|]]`, `[[Page#|Alias]]`) is not a
//! wiki construct, so the whole spelling stays plain text — it carries no
//! heading or alias to keep, and parsing it would not round-trip back to source.
//!
//! * `[[Page]]` — a link to `Page`.
//! * `[[Page#Heading]]` — a link to a heading within `Page`.
//! * `[[#Heading]]` — a same-note heading link (empty target).
//! * `[[Page|Alias]]` — a link displayed as `Alias`.
//! * `![[image.png]]` — an embed of `image.png`.
//! * `![[note#^block]]` — an embed of a block within `note`.
//!
//! This grammar is an Obsidian-inspired subset, not a full reimplementation:
//! `target`, `fragment`, and `alias` are raw strings split on the first `#` and
//! `|`, with no interpretation. Obsidian's special heading/block search forms
//! are not modeled — `[[## team]]` parses as an empty target with fragment
//! `# team`, and `[[^^block]]` as the target `^^block`. `?` is likewise an
//! ordinary `target` character. How `fragment` and `alias` are interpreted is
//! left to the consumer. The HTML compiler renders both links and embeds as an
//! `<a>` — an embed needs a resolver to inline its target, so a link is the
//! honest fallback (see [`to_html`][crate::to_html]); the mdast nodes keep the
//! full data for consumers that resolve targets themselves.
//!
//! As wiki syntax lives in the text content type, it does not trigger inside
//! code (text), code (fenced), autolinks, or other raw spans, because those are
//! tokenized first. To keep boundaries unambiguous, wiki syntax is also not
//! recognized while any link or image label opener is active — so the brackets
//! resolve to a surrounding `[…[[x]]…](url)`. This is deliberately broad: an
//! opener that never closes (`[a [[b]]`) also suppresses it, because the
//! bracket-label machinery owns those boundaries during tokenization.
//!
//! ## Tokens
//!
//! * [`WikiLink`][crate::event::Name::WikiLink]
//! * [`WikiEmbed`][crate::event::Name::WikiEmbed]
//! * [`WikiMarker`][crate::event::Name::WikiMarker]
//! * [`WikiTarget`][crate::event::Name::WikiTarget]
//! * [`WikiFragmentMarker`][crate::event::Name::WikiFragmentMarker]
//! * [`WikiFragment`][crate::event::Name::WikiFragment]
//! * [`WikiAliasMarker`][crate::event::Name::WikiAliasMarker]
//! * [`WikiAlias`][crate::event::Name::WikiAlias]
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
    if tokenizer.parse_state.options.constructs.wiki
        && tokenizer.tokenize_state.label_starts.is_empty()
        && tokenizer.current == Some(b'[')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 1) == Some(&b'[')
    {
        tokenizer.tokenize_state.token_1 = Name::WikiLink;
        tokenizer.tokenize_state.size = 2;
        tokenizer.tokenize_state.seen = false;
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
    if tokenizer.parse_state.options.constructs.wiki
        && tokenizer.tokenize_state.label_starts.is_empty()
        && tokenizer.current == Some(b'!')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 1) == Some(&b'[')
        && tokenizer.parse_state.bytes.get(tokenizer.point.index + 2) == Some(&b'[')
    {
        tokenizer.tokenize_state.token_1 = Name::WikiEmbed;
        tokenizer.tokenize_state.size = 2;
        tokenizer.tokenize_state.seen = false;
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
        State::Retry(StateName::WikiTargetStart)
    } else {
        nok(tokenizer)
    }
}

/// At the first byte of the target.
///
/// The target may be empty only when a fragment follows (a same-note link like
/// `[[#Heading]]`); a leading `|` (alias only) or the closing `]` (fully empty)
/// is not a wiki construct. Validity is finalized at [`close`][] via
/// `tokenize_state.seen`.
///
/// ```markdown
/// > | a [[b#c|d]] e
///        ^
/// ```
pub fn target_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        // Empty target, but a fragment may still follow (same-note link).
        Some(b'#') => {
            tokenizer.enter(Name::WikiFragmentMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiFragmentMarker);
            State::Next(StateName::WikiFragmentStart)
        }
        None | Some(b'\n' | b'[' | b']' | b'|') => nok(tokenizer),
        Some(_) => {
            tokenizer.tokenize_state.seen = true;
            tokenizer.enter(Name::WikiTarget);
            State::Retry(StateName::WikiTarget)
        }
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
        None | Some(b'\n' | b'[') => nok(tokenizer),
        Some(b']') => {
            tokenizer.exit(Name::WikiTarget);
            State::Retry(StateName::WikiClose)
        }
        Some(b'#') => {
            tokenizer.exit(Name::WikiTarget);
            tokenizer.enter(Name::WikiFragmentMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiFragmentMarker);
            State::Next(StateName::WikiFragmentStart)
        }
        Some(b'|') => {
            tokenizer.exit(Name::WikiTarget);
            tokenizer.enter(Name::WikiAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiAliasMarker);
            State::Next(StateName::WikiAliasStart)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::WikiTarget)
        }
    }
}

/// At the first byte of the fragment (after `#`).
///
/// The fragment must be non-empty: a `#` immediately followed by `|`/`]`
/// (`[[Page#]]`, `[[Page#|x]]`) is not a wiki construct, so the whole spelling
/// is plain text. A bare `#` carries no heading to keep, and dropping it would
/// stop the source round-tripping.
///
/// ```markdown
/// > | a [[b#c|d]] e
///          ^
/// ```
pub fn fragment_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n' | b'[' | b']' | b'|') => nok(tokenizer),
        Some(_) => {
            tokenizer.tokenize_state.seen = true;
            tokenizer.enter(Name::WikiFragment);
            State::Retry(StateName::WikiFragment)
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
        None | Some(b'\n' | b'[') => nok(tokenizer),
        Some(b']') => {
            tokenizer.exit(Name::WikiFragment);
            State::Retry(StateName::WikiClose)
        }
        Some(b'|') => {
            tokenizer.exit(Name::WikiFragment);
            tokenizer.enter(Name::WikiAliasMarker);
            tokenizer.consume();
            tokenizer.exit(Name::WikiAliasMarker);
            State::Next(StateName::WikiAliasStart)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::WikiFragment)
        }
    }
}

/// At the first byte of the alias (after `|`).
///
/// The alias must be non-empty: a `|` immediately followed by `]` (`[[Page|]]`)
/// is not a wiki construct, so the whole spelling is plain text — the same rule
/// as an empty fragment (see the module docs).
///
/// ```markdown
/// > | a [[b#c|d]] e
///            ^
/// ```
pub fn alias_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n' | b'[' | b']') => nok(tokenizer),
        Some(_) => {
            tokenizer.enter(Name::WikiAlias);
            State::Retry(StateName::WikiAlias)
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
        None | Some(b'\n' | b'[') => nok(tokenizer),
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
    // Reject a body with no non-empty target or fragment
    // (`[[]]`, `[[#]]`, `[[|x]]`).
    if !tokenizer.tokenize_state.seen {
        nok(tokenizer)
    } else if tokenizer.current == Some(b']')
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
    tokenizer.tokenize_state.seen = false;
    State::Ok
}

/// Reset state and signal failure.
fn nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.seen = false;
    State::Nok
}
