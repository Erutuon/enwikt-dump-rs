use parse_wiki_text::Positioned;

pub mod template_parameters;

pub fn get_nodes_text<'a, P: 'a + Positioned> (wikitext: &'a str, nodes: &'a Vec<P>)
    -> &'a str
{
    let (start, end) = match nodes.len() {
        0 => return "",
        1 => {
            (nodes[0].start(), nodes[0].end())
        },
        _ => {
            (nodes[0].start(), nodes[nodes.len() - 1].end())
        }
    };
    &wikitext[start..end]
}

#[cfg(test)]
mod tests {
    use super::get_nodes_text;
    use parse_wiki_text::{Configuration, Node};
    
    #[test]
    fn nodes() {
        let configuration = Configuration::default();
        
        // Test Vec<Node<'a>> with varying numbers of members.
        let examples = vec![
            ("[[title|text]]", "text"),
            ("[[title|text {{template}}]]", "text {{template}}"),
            ("[[title|text {{template}} other text]]", "text {{template}} other text"),
            
        ];
        for (link, expected_link_text) in examples {
            let parsed = configuration.parse(link);
            let link_text = parsed.nodes
                .iter()
                .find_map(|n| match n {
                    Node::Link { text, .. } => Some(get_nodes_text(&link, &text)),
                    _ => None,
                });
            assert_eq!(link_text, Some(expected_link_text));
        }
    }
    
    #[test]
    fn list_items() {
        let configuration = Configuration::default();
        
        // Test with Vec<ListItem<'a>>.
        let wikitext = concat!(
            "==Our values==\n",
            "*Correctness\n",
            "*Speed\n",
            "*Ergonomics"
        );
        let parsed = configuration.parse(wikitext);
        let list_items = parsed.nodes
            .iter()
            .find_map(|n| match n {
                Node::UnorderedList { items, .. } => Some(get_nodes_text(&wikitext, &items)),
                _ => None,
            });
        assert_eq!(list_items, Some(concat!(
            "Correctness\n",
            "*Speed\n",
            "*Ergonomics"
        )));
    }
}