//! Leaf directives occur in the [flow][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! directive_leaf ::= '::' name [label] [attributes] *space_or_tab
//! ```
//!
//! A leaf directive is exactly two colons at the start of a line, a
//! [name][crate::construct::partial_directive], an optional `[label]`, an
//! optional `{attributes}`, and then only optional whitespace until the end of
//! the line.
//!
//! ## References
//!
//! * [`directive-leaf.js` in `micromark-extension-directive`](https://github.com/micromark/micromark-extension-directive)
//!
//! [flow]: crate::construct::flow

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of a leaf directive.
///
/// ```markdown
/// > | ::name[label]{attrs}
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.directive && tokenizer.current == Some(b':') {
        tokenizer.tokenize_state.size = 0;
        tokenizer.enter(Name::DirectiveLeaf);
        tokenizer.enter(Name::DirectiveLeafSequence);
        State::Retry(StateName::DirectiveLeafSequenceOpen)
    } else {
        State::Nok
    }
}

/// In the colons of a leaf directive (exactly two).
///
/// ```markdown
/// > | ::name
///      ^
/// ```
pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b':') {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::DirectiveLeafSequenceOpen)
    } else if tokenizer.tokenize_state.size == 2 {
        tokenizer.tokenize_state.size = 0;
        tokenizer.exit(Name::DirectiveLeafSequence);
        tokenizer.attempt(
            State::Next(StateName::DirectiveLeafAfterName),
            State::Next(StateName::DirectiveLeafNok),
        );
        State::Retry(StateName::DirectiveNameStart)
    } else {
        // Not exactly two colons (a `:::` line is taken by the container, which
        // flow tries first).
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

/// After the name.
pub fn after_name(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DirectiveLeafAfterLabel),
        State::Next(StateName::DirectiveLeafAfterLabel),
    );
    State::Retry(StateName::DirectiveLabelStart)
}

/// After the optional label.
pub fn after_label(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DirectiveLeafAfterAttributes),
        State::Next(StateName::DirectiveLeafAfterAttributes),
    );
    State::Retry(StateName::DirectiveAttributesStart)
}

/// After the optional attributes.
pub fn after_attributes(_tokenizer: &mut Tokenizer) -> State {
    State::Retry(StateName::DirectiveLeafEnd)
}

/// At the end of the line.
///
/// Only optional whitespace may follow the attributes.
///
/// ```markdown
/// > | ::name
///           ^
/// ```
pub fn end(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::DirectiveLeafEnd)
        }
        None | Some(b'\n') => {
            tokenizer.exit(Name::DirectiveLeaf);
            State::Ok
        }
        _ => State::Nok,
    }
}

/// At a failure.
pub fn nok(_tokenizer: &mut Tokenizer) -> State {
    State::Nok
}
