use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    ops::{Index, IndexMut},
};
use serde::{
    Serialize,
    ser::Serializer,
};
use wiktionary_namespaces::Namespace;
use dump_parser::{
    DumpParser,
    wiktionary_configuration as create_configuration,
    Node::{self, *},
    Page,
    Positioned,
    Warning,
};

type HeaderLevel = u8;

const MAX_HEADER_LEVEL: usize = 6;
const MIN_HEADER_LEVEL: usize = 1;
const HEADER_LEVEL_ARRAY_SIZE: usize = MAX_HEADER_LEVEL - MIN_HEADER_LEVEL + 1;

#[derive(Debug, Serialize)]
pub struct HeaderCounts(
    [usize; HEADER_LEVEL_ARRAY_SIZE]
);

impl HeaderCounts {
    fn new() -> Self {
        HeaderCounts([0usize; HEADER_LEVEL_ARRAY_SIZE])
    }
}

impl Index<HeaderLevel> for HeaderCounts {
    type Output = usize;
    
    fn index<'a> (&'a self, index: HeaderLevel) -> &'a Self::Output {
        &self.0[index as usize - MIN_HEADER_LEVEL]
    }
}

impl IndexMut<HeaderLevel> for HeaderCounts {
    fn index_mut<'a> (&'a mut self, index: HeaderLevel) -> &'a mut Self::Output {
        &mut self.0[index as usize - MIN_HEADER_LEVEL]
    }
}

impl Serialize for HeaderStats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        #[derive(Serialize)]
        struct HeaderStat<'a> {
            header: &'a str,
            counts: &'a HeaderCounts,
        }
        
        let mut header_counts: Vec<_> = self.header_counts
            .iter()
            .map(|(header, counts)| HeaderStat { header, counts })
            .collect();
        header_counts.sort_by(|HeaderStat { header: header1, .. }, HeaderStat { header: header2, .. }| {
            header1.cmp(header2)
        });
        header_counts.serialize(serializer)
    }
}

#[derive(Debug)]
pub struct HeaderStats {
    pub header_counts: HashMap<String, HeaderCounts>,
}

impl HeaderStats {
    #[inline]
    pub fn new() -> Self {
        Self { header_counts: HashMap::new() }
    }
    
    pub fn parse (
        &mut self,
        parser: DumpParser,
        page_limit: usize,
        namespaces: Vec<Namespace>,
        verbose: bool,
    ) {
        let namespaces: HashSet<Namespace> = namespaces.into_iter().collect();
        let parser = parser
            .map(|result| {
                result.unwrap_or_else(|e| {
                    panic!("Error while parsing dump: {}", e);
                })
            })
            .filter(|page| {
                namespaces.contains(&page.namespace.try_into().unwrap())
            })
            .take(page_limit);
        let configuration = create_configuration();
        for page in parser {
            // eprintln!("title: [[{}]]", &page.title);
            let parser_output = configuration.parse(&page.text);
            if verbose {
                for warning in parser_output.warnings {
                    let Warning { start, end, message } = warning;
                    let range = 0..page.text.len();
                    let message = message.message().trim_end_matches(".");
                    if !(range.contains(&start) && range.contains(&end)) {
                        eprintln!("byte position {} or {} in warning {} is out of range of {:?}, size of [[{}]]",
                            start, end, message, range, &page.title);
                    } else {
                        eprintln!("{} at bytes {}..{} ({:?}) in [[{}]]",
                            &message,
                            start, end, &page.text[start..end], &page.title);
                    }
                }
            }
            
            self.process_nodes(&page, &parser_output.nodes);
        }
    }

    fn process_nodes (
        &mut self,
        page: &Page,
        nodes: &Vec<Node>,
    ) {
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.process_nodes(&page, &item.nodes);
                    }
                },
                Heading { nodes, level, .. } => {
                    self.process_header(&page, &nodes, *level);
                },
                  Preformatted { nodes, .. }
                | Tag { nodes, .. } => {
                    self.process_nodes(&page, &nodes);
                },
                  Image { text, .. }
                | Link { text, .. } => {
                    self.process_nodes(&page, &text);
                },
                  OrderedList { items, .. }
                | UnorderedList { items, .. } => {
                    for item in items {
                        self.process_nodes(&page, &item.nodes);
                    }
                },
                Parameter { name, default, .. } => {
                    match default {
                        Some(nodes) => self.process_nodes(&page, &nodes),
                        None => {},
                    }
                    self.process_nodes(&page, &name);
                },
                Table { attributes, captions, rows, .. } => {
                    self.process_nodes(&page, &attributes);
                    for caption in captions {
                        if let Some(attributes) = &caption.attributes {
                            self.process_nodes(&page, attributes)
                        }
                        self.process_nodes(&page, &caption.content);
                    }
                    for row in rows {
                        self.process_nodes(&page, &row.attributes);
                        for cell in &row.cells {
                            if let Some(attributes) = &cell.attributes {
                                self.process_nodes(&page, attributes);
                            }
                            self.process_nodes(&page, &cell.content);
                        }
                    }
                },
                Template { name, parameters, .. } => {
                    self.process_nodes(&page, &name);
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.process_nodes(&page, name);
                        }
                        self.process_nodes(&page, &parameter.value);
                    }
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
                | Text {..} => {},
            }
        }
    }

    fn process_header(
        &mut self,
        page: &Page,
        nodes: &Vec<Node>,
        level: u8,
    ) {
        let key = nodes.get_text_from(&page.text)
            .trim_matches(|c: char| c == ' ' || c == '\t');
        let value = self.header_counts.entry(key.into())
            .or_insert_with(|| HeaderCounts::new());
        *&mut value[level as HeaderLevel] += 1;
    }
}