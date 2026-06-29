//! No JS equivalent: wiki embeds are this fork's own node.

use super::wiki_link::serialize_wiki;
use super::Handle;
use crate::state::{Info, State};
use alloc::string::String;
use markdown::{
    mdast::{Node, WikiEmbed},
    message::Message,
};

impl Handle for WikiEmbed {
    fn handle(
        &self,
        _state: &mut State,
        _info: &Info,
        _parent: Option<&Node>,
        _node: &Node,
    ) -> Result<String, Message> {
        serialize_wiki(
            true,
            &self.target,
            self.fragment.as_deref(),
            self.alias.as_deref(),
        )
    }
}
