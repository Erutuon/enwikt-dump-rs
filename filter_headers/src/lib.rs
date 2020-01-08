use dump_parser::{
    wiktionary_configuration as create_configuration, DumpParser,
    Node::{self, *},
    Page, Positioned, Warning,
};
use serde::{Serialize, Serializer};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    io::Read,
};
use wiktionary_namespaces::Namespace;

#[derive(Debug)]
pub struct HeaderFilterer {
    top_level_headers: HashSet<String>,
    other_headers: HashSet<String>,
    header_to_titles: HashMap<String, HashSet<String>>,
}

#[derive(Serialize)]
struct Entry<'a> {
    header: &'a str,
    titles: Vec<&'a String>,
}

impl Serialize for HeaderFilterer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut header_to_titles: Vec<_> = self
            .header_to_titles
            .iter()
            .map(|(header, titles)| {
                let mut titles: Vec<&String> = titles.iter().collect();
                titles.sort();
                Entry { header, titles }
            })
            .collect();
        header_to_titles.sort_by(
            |Entry {
                 header: header1, ..
             },
             Entry {
                 header: header2, ..
             }| { header1.cmp(header2) },
        );
        header_to_titles.serialize(serializer)
    }
}

impl HeaderFilterer {
    pub fn new(
        top_level_headers: Vec<String>,
        other_headers: Vec<String>,
    ) -> Self {
        Self {
            top_level_headers: top_level_headers.into_iter().collect(),
            other_headers: other_headers.into_iter().collect(),
            header_to_titles: HashMap::new(),
        }
    }

    pub fn parse<R: Read>(
        &mut self,
        parser: DumpParser<R>,
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
            let parser_output = configuration.parse(&page.text);
            if verbose {
                for warning in parser_output.warnings {
                    let Warning {
                        start,
                        end,
                        message,
                    } = warning;
                    let range = 0..page.text.len();
                    let message = message.message().trim_end_matches(".");
                    if !(range.contains(&start) && range.contains(&end)) {
                        eprintln!("byte position {} or {} in warning {} is out of range of {:?}, size of [[{}]]",
                            start, end, message, range, &page.title);
                    } else {
                        eprintln!(
                            "{} at bytes {}..{} ({:?}) in [[{}]]",
                            &message,
                            start,
                            end,
                            &page.text[start..end],
                            &page.title
                        );
                    }
                }
            }

            self.process_nodes(&page, &parser_output.nodes);
        }
    }

    fn process_nodes(&mut self, page: &Page, nodes: &Vec<Node>) {
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.process_nodes(&page, &item.nodes);
                    }
                }
                Heading { nodes, level, .. } => {
                    self.process_header(&page, &nodes, *level);
                }
                Preformatted { nodes, .. } | Tag { nodes, .. } => {
                    self.process_nodes(&page, &nodes);
                }
                Image { text, .. } | Link { text, .. } => {
                    self.process_nodes(&page, &text);
                }
                OrderedList { items, .. } | UnorderedList { items, .. } => {
                    for item in items {
                        self.process_nodes(&page, &item.nodes);
                    }
                }
                Parameter { name, default, .. } => {
                    match default {
                        Some(nodes) => self.process_nodes(&page, &nodes),
                        None => {}
                    }
                    self.process_nodes(&page, &name);
                }
                Table {
                    attributes,
                    captions,
                    rows,
                    ..
                } => {
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
                }
                Template {
                    name, parameters, ..
                } => {
                    self.process_nodes(&page, &name);
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.process_nodes(&page, name);
                        }
                        self.process_nodes(&page, &parameter.value);
                    }
                }
                Bold { .. }
                | BoldItalic { .. }
                | Category { .. }
                | CharacterEntity { .. }
                | Comment { .. }
                | EndTag { .. }
                | ExternalLink { .. }
                | HorizontalDivider { .. }
                | Italic { .. }
                | MagicWord { .. }
                | ParagraphBreak { .. }
                | Redirect { .. }
                | StartTag { .. }
                | Text { .. } => {}
            }
        }
    }

    fn process_header(&mut self, page: &Page, nodes: &Vec<Node>, level: u8) {
        let text = nodes
            .get_text_from(&page.text)
            .trim_matches(|c: char| c == ' ' || c == '\t');
        if !match level {
            2 => &self.top_level_headers,
            _ => &self.other_headers,
        }
        .contains(text)
        {
            let titles = self
                .header_to_titles
                .entry(text.into())
                .or_insert_with(|| HashSet::new());
            titles.insert(page.title.to_string());
        }
    }
}
