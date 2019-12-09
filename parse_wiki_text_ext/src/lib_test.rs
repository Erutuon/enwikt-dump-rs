use std::ops::Range;
use parse_wiki_text::Positioned;

pub mod template_parameters;

pub trait PositionedExt {
    /// The range of byte positions in the wiki text where the element starts and ends.
    fn range(&self) -> Range<usize>;
    
    /// Retrieve the wiki text for the element by byte range.
    fn get_text_from<'a>(&self, text: &'a str) -> &'a str;
}

impl<P: Positioned> PositionedExt for P {
    /// The range of byte positions in the wiki text where the element starts and ends.
    fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }
    
    /// Retrieve the wiki text for the element by byte range.
    fn get_text_from<'a>(&self, text: &'a str) -> &'a str {
        &text[self.range()]
    }
}

fn end<P: Positioned>(vec: &Vec<P>) -> usize {
    if let Some(node) = vec.last() {
        node.end()
    } else {
        0
    }
}

fn start<P: Positioned>(vec: &Vec<P>) -> usize {
    if let Some(node) = vec.first() {
        node.start()
    } else {
        0
    }
}

impl<P: Positioned> PositionedExt for Vec<P> {
    /// The range of byte positions in the wiki text where the element starts and ends.
    fn range(&self) -> Range<usize> {
        start(self)..end(self)
    }
    
    /// Retrieve the wiki text for the element by byte range.
    fn get_text_from<'a>(&self, text: &'a str) -> &'a str {
        &text[self.range()]
    }
}

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
            ("[[title|text {{template}} other text]]", "text {{template}} other text"),
            
        ];
        for (link, expected_link_text) in examples {
            let parsed = configuration.parse(link);
            let link_text = parsed.nodes
                .iter()
                .find_map(|n| match n {
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
        let list_items = parsed.nodes
            .iter()
            .find_map(|n| match n {
                Node::UnorderedList { items, .. } => Some(&items.get_text_from(&wikitext)),
                _ => None,
            });
        assert_eq!(list_items, Some(concat!(
            "Correctness\n",
            "*Speed\n",
            "*Ergonomics"
        )));
    }
}