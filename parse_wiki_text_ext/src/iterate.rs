// Doesn't work because each iterator has a different type!
use parse_wiki_text::Node::{self, *};

pub fn iter_subnodes<'a> (node: &'a Node) -> Option<impl Iterator<Item = &'a Node<'a>>> {
    let iter = match node {
          OrderedList { items, .. }
        | UnorderedList { items, .. } => {
            items
                .iter()
                .flat_map(|item| {
                    item.nodes.iter()
                })
        },
        DefinitionList { items, .. } => {
            items
                .iter()
                .flat_map(|item| {
                    item.nodes.iter()
                })
        },
          Heading { nodes, .. }
        | Preformatted { nodes, .. }
        | Tag { nodes, .. } => nodes.iter(),
          Image { text, .. }
        | Link { text, .. } => text.iter(),
        Parameter { name, default, .. } => {
            let iter = name.iter();
            match default {
                Some(nodes) => iter.chain(nodes.iter()),
                None => iter,
            }
        },
        Table { attributes, captions, rows, .. } => {
            attributes
                .iter()
                .chain(
                    captions
                        .iter()
                        .flat_map(|caption| {
                            if let Some(attributes) = &caption.attributes {
                                attributes.iter().chain(caption.content.iter())
                            } else {
                                caption.content.iter()
                            }
                        })
                )
                .chain(
                    rows
                        .iter()
                        .flat_map(|row| {
                            row.attributes
                                .iter()
                                .chain(row.cells
                                    .iter()
                                    .flat_map(|cell| {
                                        if let Some(attributes) = &cell.attributes {
                                            attributes
                                                .iter()
                                                .chain(
                                                    cell.content.iter())
                                        } else {
                                            cell.content.iter()
                                        }
                                    })
                                )
                        })
                )
        },
        Template { name, parameters, .. } => {
            name
                .iter()
                .chain(
                    parameters
                        .iter()
                        .flat_map(|parameter| {
                            if let Some(name) = &parameter.name {
                                name
                                    .iter()
                                    .chain(
                                        parameter.value
                                        .iter()
                                    )
                            } else {
                                parameter.value.iter()
                            }
                        })
                )
        },
          Bold {..}
        | BoldItalic {..}
        | Category {..}
        | CharacterEntity {..}
        | Comment {..}
        | EndTag {..}
        | ExternalLink {..}
        | HorizontalDivider {..}
        | Italic {..}
        | MagicWord {..}
        | ParagraphBreak {..}
        | Redirect {..}
        | StartTag {..}
        | Text {..} => return None,
    };
    Some(iter)
}