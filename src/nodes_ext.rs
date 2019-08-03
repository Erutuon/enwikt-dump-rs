use parse_wiki_text::Positioned;

fn get_start_and_end<P: Positioned> (node: &P) -> (usize, usize) {
    (node.start(), node.end())
}

pub fn get_nodes_text<'a, P: 'a + Positioned> (wikitext: &'a str, nodes: &'a Vec<P>)
    -> &'a str
{
    let (start, end) = match nodes.into_iter().count() {
        0 => return "",
        1 => {
            get_start_and_end(nodes.into_iter().nth(0).unwrap())
        },
        _ => {
            let (start, _) = get_start_and_end(nodes.into_iter().nth(0).unwrap());
            let (_, end) = get_start_and_end(nodes.into_iter().last().unwrap());
            (start, end)
        }
    };
    &wikitext[start..end]
}

/*
fn get_start_and_end (node: &Node) -> (usize, usize) {
    match node {
          DefinitionList { start, end, .. }
        | Heading { start, end, .. }
        | Image { start, end, .. }
        | Link { start, end, .. }
        | OrderedList { start, end, .. }
        | Parameter { start, end, .. }
        | Preformatted { start, end, .. }
        | Table { start, end, .. }
        | Tag { start, end, .. }
        | Template { start, end, .. }
        | UnorderedList { start, end, .. }
        | Bold { start, end, .. }
        | BoldItalic { start, end, .. }
        | Category { start, end, .. }
        | CharacterEntity { start, end, .. }
        | Comment { start, end, .. }
        | EndTag { start, end, .. }
        | ExternalLink { start, end, .. }
        | HorizontalDivider { start, end, .. }
        | Italic { start, end, .. }
        | MagicWord { start, end, .. }
        | ParagraphBreak { start, end, .. }
        | Redirect { start, end, .. }
        | StartTag { start, end, .. }
        | Text { start, end, .. } => (*start, *end),
    }
}

pub fn get_nodes_text<'a> (wikitext: &'a str, nodes: &'a Vec<Node>) -> &'a str {
    let (start, end) = match nodes.len() {
        0 => return "",
        1 => {
            get_start_and_end(&nodes[0])
        },
        _ => {
            let (start, _) = get_start_and_end(&nodes[0]);
            let (_, end) = get_start_and_end(&nodes.last().unwrap());
            (start, end)
        }
    };
    &wikitext[start..end]
}
*/