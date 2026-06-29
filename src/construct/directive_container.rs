//! Container directives occur in the [flow][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! opening_fence ::= 3*':' name [label] [attributes] *space_or_tab
//! closing_fence ::= *space_or_tab 3*':' *space_or_tab
//! ```
//!
//! A container directive is a fenced block: an opening fence of three or more
//! colons followed by a [name][crate::construct::partial_directive] (and
//! optional `[label]`/`{attributes}`), some flow content, and a closing fence
//! of only colons (at least as many as the opening fence).
//!
//! ## Parser strategy
//!
//! Unlike `micromark-extension-directive`, container directives here are *not*
//! a single construct that owns its content. Instead, the opening fence and the
//! closing fence are each emitted as their own [`DirectiveContainerFence`][df]
//! flow event, and the content in between is parsed by the normal document/flow
//! machinery. The nesting is reconstructed in [`to_mdast`][crate::to_mdast]
//! (the same approach this codebase uses for MDX JSX flow elements).
//!
//! This is what makes the boundary cases resolve correctly: because every line
//! is parsed independently as flow, a list, block quote, or nested directive
//! inside a container Just Works, and the fences interrupt paragraphs and lists
//! like any other flow construct.
//!
//! EOF auto-close is handled in `to_mdast`: any container still open at the end
//! of the document is closed there.
//!
//! [flow]: crate::construct::flow
//! [df]: crate::event::Name::DirectiveContainerFence

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of a container directive fence (opening or closing).
///
/// ```markdown
/// > | :::name
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.directive && tokenizer.current == Some(b':') {
        tokenizer.tokenize_state.size = 0;
        tokenizer.enter(Name::DirectiveContainerFence);
        tokenizer.enter(Name::DirectiveContainerSequence);
        State::Retry(StateName::DirectiveContainerSequenceOpen)
    } else {
        State::Nok
    }
}

/// In the opening sequence of colons.
///
/// ```markdown
/// > | :::name
///     ^^^
/// ```
pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b':') {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::DirectiveContainerSequenceOpen)
    } else if tokenizer.tokenize_state.size >= 3 {
        tokenizer.exit(Name::DirectiveContainerSequence);
        // A name means this is an opening fence; otherwise it may be a closing
        // fence (a line of only colons).
        tokenizer.attempt(
            State::Next(StateName::DirectiveContainerAfterName),
            State::Next(StateName::DirectiveContainerCloseStart),
        );
        State::Retry(StateName::DirectiveNameStart)
    } else {
        // Fewer than three colons: not a container (leaf/text handle it).
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

/// After the name (opening fence).
pub fn after_name(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DirectiveContainerAfterLabel),
        State::Next(StateName::DirectiveContainerAfterLabel),
    );
    State::Retry(StateName::DirectiveLabelStart)
}

/// After the optional label (opening fence).
pub fn after_label(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DirectiveContainerAfterAttributes),
        State::Next(StateName::DirectiveContainerAfterAttributes),
    );
    State::Retry(StateName::DirectiveAttributesStart)
}

/// After the optional attributes (opening fence).
pub fn after_attributes(_tokenizer: &mut Tokenizer) -> State {
    State::Retry(StateName::DirectiveContainerOpenAfter)
}

/// At the end of an opening fence line.
///
/// Only optional whitespace may follow.
///
/// ```markdown
/// > | :::name
///            ^
/// ```
pub fn open_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::DirectiveContainerOpenAfter)
        }
        None | Some(b'\n') => {
            tokenizer.exit(Name::DirectiveContainerFence);
            tokenizer.tokenize_state.size = 0;
            State::Ok
        }
        // Junk after the attributes: not a valid opening fence.
        _ => {
            tokenizer.tokenize_state.size = 0;
            State::Nok
        }
    }
}

/// At a closing fence candidate (no name after the colons).
///
/// ```markdown
/// > | :::
///        ^
/// ```
pub fn close_start(_tokenizer: &mut Tokenizer) -> State {
    State::Retry(StateName::DirectiveContainerCloseSequence)
}

/// On a closing fence line, before optional trailing whitespace.
pub fn close_sequence(_tokenizer: &mut Tokenizer) -> State {
    State::Retry(StateName::DirectiveContainerCloseAfter)
}

/// At the end of a closing fence line.
///
/// Only optional whitespace may follow the colons.
///
/// ```markdown
/// > | :::
///        ^
/// ```
pub fn close_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::DirectiveContainerCloseAfter)
        }
        None | Some(b'\n') => {
            tokenizer.exit(Name::DirectiveContainerFence);
            tokenizer.tokenize_state.size = 0;
            State::Ok
        }
        // Junk after the colons: not a valid closing fence.
        _ => {
            tokenizer.tokenize_state.size = 0;
            State::Nok
        }
    }
}
