use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::TryInto,
    fs::File,
    io::{BufWriter, Write},
    rc::Rc,
};
use parse_wiki_text::{self, Node::{self, *}, Warning};
use parse_mediawiki_dump::Page;
use crate::{
    dump_parser::DumpParser,
    namespace::Namespace,
};

use crate::nodes_ext::get_nodes_text;

use crate::dump_parser;
use dump_parser::wiktionary_configuration as create_configuration;

type ShareableFileWriter = Rc<RefCell<BufWriter<File>>>;

const CHAR_BEFORE_TITLE: char = '\u{1}';
const CHAR_BEFORE_TEMPLATE: char = '\n';
const MAX_TEMPLATE_NAME: usize = 256;

fn is_template_name_whitespace(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'_'
}

pub fn normalize_template_name<'a>(name: &str, name_buffer: &'a mut [u8]) -> Option<&'a [u8]> {
    match name.bytes()
        .position(|b| !is_template_name_whitespace(b)) {
        Some(start_index) => {
            // This can't fail because finding the start index proves that
            // there's a non-whitespace character in the template name.
            let end_index = name.bytes()
                .rposition(|b| !is_template_name_whitespace(b))
                .unwrap() + 1;
            let name_buffer = &mut name_buffer[0..end_index - start_index];
            name_buffer.copy_from_slice(&name[start_index..end_index].as_bytes());
            for c in name_buffer.iter_mut() {
                if *c == b'_' {
                    *c = b' ';
                }
            }
            Some(name_buffer)
        },
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_template_name, MAX_TEMPLATE_NAME};
    
    #[test]
    fn test_normalize_template_name() {
        let mut buffer = [0u8; MAX_TEMPLATE_NAME];
        
        let name = "__test_test__  \t";
        assert_eq!(&normalize_template_name(name, &mut buffer).unwrap(), &b"test test");
        
        let name = "test test\u{a0}";
        // This is actually an invalid template name.
        assert_eq!(&normalize_template_name(name, &mut buffer).unwrap(), &name.as_bytes());
    }
}

#[derive(Debug)]
pub struct TemplateDumper {
    template_to_file: HashMap<String, (ShareableFileWriter, usize)>,
    title_printed: Vec<bool>,
}

impl TemplateDumper {
    pub fn new (template_to_file: Vec<(String, Option<String>)>) -> Self {
        let mut files: HashMap<String, (ShareableFileWriter, usize)> = HashMap::new();
        let mut file_number: usize = 0;
        let template_to_file = template_to_file
            .into_iter()
            .filter_map(|(template, filepath)| {
                if template.len() > MAX_TEMPLATE_NAME {
                    None
                } else {
                    let mut template_name = [0u8; MAX_TEMPLATE_NAME];
                    let normalized = match normalize_template_name(&template, &mut template_name) {
                        Some(n) => n,
                        None => return None,
                    };
                    let filepath = match filepath {
                        Some(p) => p.to_string(),
                        None => unsafe { std::str::from_utf8_unchecked(&normalized).to_string() + ".txt" },
                    };
                    Some((
                        unsafe { std::str::from_utf8_unchecked(&normalized).to_string() },
                        match files.get(&filepath) {
                            Some(f) => f.clone(),
                            None => {
                                let file = File::create(&filepath).unwrap_or_else(|e| {
                                    panic!("error while creating file {}: {}", &filepath, e);
                                });
                                let file_ref = Rc::new(RefCell::new(BufWriter::new(file)));
                                let file_and_number = (file_ref, file_number);
                                file_number += 1;
                                let cloned = file_and_number.clone();
                                files.insert(filepath, cloned);
                                file_and_number
                            }
                        }
                    ))
                }
            })
            .collect();
        
        let title_printed = vec![false; file_number];
        
        Self { template_to_file, title_printed }
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
                namespaces.contains(
                    &page.namespace.try_into().unwrap()
                )
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
            
            for item in &mut self.title_printed {
                *item = false;
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
            // println!("{:?}", node);
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.process_nodes(&page, &item.nodes);
                    }
                },
                  Heading { nodes, .. }
                | Preformatted { nodes, .. }
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
                    self.dump(&page, &node);
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
    
    // Todo: Normalize template name.
    fn dump (&mut self, page: &Page, template: &Node) {
        if let Template { start, end, name, .. } = template {
            let name = get_nodes_text(&page.text, &name);
            if name.len() <= MAX_TEMPLATE_NAME {
                let mut name_normalized = [0u8; MAX_TEMPLATE_NAME];
                let name = match normalize_template_name(name, &mut name_normalized) {
                    Some(n) => n,
                    None => return,
                };
                if let Some((file, number)) = self.template_to_file.get(
                    unsafe { std::str::from_utf8_unchecked(&name) }
                ) {
                    let mut file = file.borrow_mut();
                    if !self.title_printed[*number] {
                        write!(*file, "{}{}", CHAR_BEFORE_TITLE, &page.title)
                            .unwrap_or_else(|e| panic!("error while writing: {}", e));
                        self.title_printed[*number] = true;
                    }
                    write!(*file, "{}{}", CHAR_BEFORE_TEMPLATE, &page.text[*start..*end])
                        .unwrap_or_else(|e| panic!("error while writing: {}", e));
                }
            }
        }
    }
}