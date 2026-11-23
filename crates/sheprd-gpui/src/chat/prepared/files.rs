//! Extract file cards from parsed paragraphs, so Markdown reference definitions
//! are resolved before any prose is separated from a card.

use crate::chat::{
    prepared::Part,
    rich::{Block, Inline, Kind},
};
use crate::terminal::{RowTarget, file_target};

pub(super) fn split(blocks: Vec<Block>, remaining: &mut usize) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut prose = Vec::new();
    for block in blocks {
        if *remaining > 0
            && let Some(destination) = destination(&block)
        {
            let destination = destination.to_owned();
            if !prose.is_empty() {
                parts.push(Part::Rich(std::mem::take(&mut prose).into()));
            }
            parts.push(Part::File {
                id: crate::chat::visual::VisualId::of(("file-thumbnail", &destination)),
                destination,
                blocks: vec![block].into(),
            });
            *remaining -= 1;
        } else {
            prose.push(block);
        }
    }
    if !prose.is_empty() || parts.is_empty() {
        parts.push(Part::Rich(prose.into()));
    }
    parts
}

fn destination(block: &Block) -> Option<&str> {
    let Block::Paragraph {
        kind: Kind::Paragraph,
        indent: 0,
        content,
    } = block
    else {
        return None;
    };
    let mut destination = None;
    for part in content {
        let Inline::Text(_, style) = part else {
            return None;
        };
        let link = style.link.as_deref()?;
        if destination.is_some_and(|known| known != link) {
            return None;
        }
        destination = Some(link);
    }
    let destination = destination?;
    let RowTarget::Path { line: None, .. } = file_target(destination)? else {
        return None;
    };
    Some(destination)
}
