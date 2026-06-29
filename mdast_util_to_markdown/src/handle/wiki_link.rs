//! No JS equivalent: wiki links are this fork's own node.

use super::Handle;
use crate::state::{Info, State};
use alloc::{boxed::Box, format, string::String};
use markdown::{
    mdast::{Node, WikiLink},
    message::Message,
};

impl Handle for WikiLink {
    fn handle(
        &self,
        _state: &mut State,
        _info: &Info,
        _parent: Option<&Node>,
        _node: &Node,
    ) -> Result<String, Message> {
        serialize_wiki(
            false,
            &self.target,
            self.fragment.as_deref(),
            self.alias.as_deref(),
        )
    }
}

/// Serialize a wiki link or embed back to `[[target#fragment|alias]]` (with a
/// leading `!` for an embed).
///
/// Wiki syntax has no escape mechanism, so a field whose value would change the
/// parse cannot be represented. Rather than emit markdown that reparses to a
/// different tree, [`validate_wiki`] rejects such a node first. Parser-produced
/// nodes always pass; only nodes built by hand (the fields are public) can
/// fail.
pub(crate) fn serialize_wiki(
    embed: bool,
    target: &str,
    fragment: Option<&str>,
    alias: Option<&str>,
) -> Result<String, Message> {
    validate_wiki(target, fragment, alias)?;

    let mut value = String::new();
    if embed {
        value.push('!');
    }
    value.push_str("[[");
    value.push_str(target);
    if let Some(fragment) = fragment {
        value.push('#');
        value.push_str(fragment);
    }
    if let Some(alias) = alias {
        value.push('|');
        value.push_str(alias);
    }
    value.push_str("]]");
    Ok(value)
}

/// Reject a wiki node whose fields would not round-trip through the parser.
///
/// The grammar is `[[ target ['#' fragment] ['|' alias] ]]` on a single line
/// with no escaping, so:
///
/// * no field may carry a bracket (`[`/`]`) or line ending — it would break the
///   `[[ … ]]` framing;
/// * the target may not carry `#`/`|` — they would open a fragment or alias;
/// * the fragment may not carry `|` — it would open an alias;
/// * an empty fragment/alias reparses as plain text, not a wiki node, so it
///   must be `None`, not `Some("")`;
/// * the target or the fragment must be non-empty, else the spelling is text.
fn validate_wiki(target: &str, fragment: Option<&str>, alias: Option<&str>) -> Result<(), Message> {
    let breaks_frame = |c: char| matches!(c, '[' | ']' | '\n' | '\r');
    for (label, value) in [
        ("target", Some(target)),
        ("fragment", fragment),
        ("alias", alias),
    ] {
        if let Some(c) = value.and_then(|value| value.chars().find(|&c| breaks_frame(c))) {
            return Err(wiki_error(format!(
                "wiki {} cannot contain {:?}: it would break `[[ … ]]`",
                label, c
            )));
        }
    }

    if let Some(c) = target.chars().find(|&c| matches!(c, '#' | '|')) {
        return Err(wiki_error(format!(
            "wiki target cannot contain {:?}: it would open a fragment or alias",
            c
        )));
    }

    if let Some(fragment) = fragment {
        if fragment.contains('|') {
            return Err(wiki_error(
                "wiki fragment cannot contain '|': it would open an alias".into(),
            ));
        }
        if fragment.is_empty() {
            return Err(wiki_error(
                "wiki fragment cannot be empty: use `None`, a bare `#` reparses as plain text"
                    .into(),
            ));
        }
    }

    if matches!(alias, Some("")) {
        return Err(wiki_error(
            "wiki alias cannot be empty: use `None`, a bare `|` reparses as plain text".into(),
        ));
    }

    if target.is_empty() && fragment.is_none() {
        return Err(wiki_error(
            "wiki link needs a non-empty target or fragment".into(),
        ));
    }

    Ok(())
}

/// Build the error for an unrepresentable wiki node.
fn wiki_error(reason: String) -> Message {
    Message {
        place: None,
        reason,
        rule_id: Box::new("unrepresentable-wiki".into()),
        source: Box::new("mdast-util-to-markdown".into()),
    }
}
