//! Text directives occur in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! directive_text ::= ':' name [label] [attributes]
//! ```
//!
//! A text directive is a single colon, immediately followed by a
//! [name][crate::construct::partial_directive], then an optional
//! `[label]` and optional `{attributes}`.
//!
//! The colon may not be preceded by another (unescaped) colon, so `a ::b`
//! does not form a directive.
//!
//! ## References
//!
//! * [`directive-text.js` in `micromark-extension-directive`](https://github.com/micromark/micromark-extension-directive)
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start of a text directive.
///
/// ```markdown
/// > | a :b[c]{d=e} f
///       ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.parse_state.options.constructs.directive
        && tokenizer.current == Some(b':')
        // Not preceded by an (unescaped) colon.
        && tokenizer.previous != Some(b':')
    {
        tokenizer.enter(Name::DirectiveText);
        tokenizer.enter(Name::DirectiveTextMarker);
        tokenizer.consume();
        State::Next(StateName::DirectiveTextAfterMarker)
    } else {
        State::Nok
    }
}

/// After the colon marker.
///
/// ```markdown
/// > | a :b[c]{d=e} f
///        ^
/// ```
pub fn after_marker(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::DirectiveTextMarker);
    tokenizer.attempt(
        State::Next(StateName::DirectiveTextAfterName),
        State::Next(StateName::DirectiveTextNok),
    );
    State::Retry(StateName::DirectiveNameStart)
}

/// After the name.
///
/// ```markdown
/// > | a :b[c]{d=e} f
///         ^
/// ```
pub fn after_name(tokenizer: &mut Tokenizer) -> State {
    // `:b:` is not a directive.
    if tokenizer.current == Some(b':') {
        State::Nok
    } else {
        tokenizer.attempt(
            State::Next(StateName::DirectiveTextAfterLabel),
            State::Next(StateName::DirectiveTextAfterLabel),
        );
        State::Retry(StateName::DirectiveLabelStart)
    }
}

/// After the optional label.
///
/// ```markdown
/// > | a :b[c]{d=e} f
///            ^
/// ```
pub fn after_label(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DirectiveTextAfterAttributes),
        State::Next(StateName::DirectiveTextAfterAttributes),
    );
    State::Retry(StateName::DirectiveAttributesStart)
}

/// After the optional attributes.
///
/// ```markdown
/// > | a :b[c]{d=e} f
///                ^
/// ```
pub fn after_attributes(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::DirectiveText);
    State::Ok
}

/// At a failure (no valid name).
pub fn nok(_tokenizer: &mut Tokenizer) -> State {
    State::Nok
}
