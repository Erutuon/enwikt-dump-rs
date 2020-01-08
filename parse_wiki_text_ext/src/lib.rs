pub mod template_parameters;

#[cfg(test)]
mod tests {
    use parse_wiki_text::{Configuration, Node};

    #[test]
    fn nodes() {
        let configuration = Configuration::default();

        // Test Vec<Node<'a>> with varying numbers of members.
        let examples = vec![
            ("[[title|text]]", "text"),
            ("[[title|text {{template}}]]", "text {{template}}"),
            (
                "[[title|text {{template}} other text]]",
                "text {{template}} other text",
            ),
        ];
        for (link, expected_link_text) in examples {
            let parsed = configuration.parse(link);
            let link_text = parsed.nodes.iter().find_map(|n| match n {
                Node::Link { text, .. } => Some(&text.get_text_from(&link)),
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
        let list_items = parsed.nodes.iter().find_map(|n| match n {
            Node::UnorderedList { items, .. } => {
                Some(&items.get_text_from(&wikitext))
            }
            _ => None,
        });
        assert_eq!(
            list_items,
            Some(concat!("Correctness\n", "*Speed\n", "*Ergonomics"))
        );
    }
}
