use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::TryInto,
    fs::File,
    io::{BufWriter, Write},
    rc::Rc,
};
use mediawiki::api::Api as MediaWikiApi;
use serde_json::Value;
use dump_parser::{
    DumpParser,
    Node::{self, *},
    Page,
    Positioned,
    Warning,
    wiktionary_configuration as create_configuration,
};
use wiktionary_namespaces::Namespace;

enum QueryKey {
    Redirects,
    Normalized,
    Pages,
}

impl QueryKey {
    fn as_str(&self) -> &'static str {
        match self {
            QueryKey::Redirects => "redirects",
            QueryKey::Normalized => "normalized",
            QueryKey::Pages => "pages",
        }
    }
}

#[derive(Debug)]
pub struct TemplateRedirects {
    query_responses: Vec<Value>,
    // template_to_redirects: HashMap<&'a str, Option<Vec<&'a str>>>,
    // redirects_followed: Option<HashMap<&'a str, &'a str>>,
}

// Idea: indicate whether template exists and has no redirects, or is nonexistent.
impl<'a> TemplateRedirects {
    pub fn new<I, S> (templates: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: std::string::ToString,
    {
        let api = MediaWikiApi::new("https://en.wiktionary.org/w/api.php").unwrap();
        
        let templates: Vec<_> = templates
            .into_iter()
            .map(|t| "Template:".to_string() + &t.to_string())
            .collect();
        
        let mut query_responses = Vec::new();
        
        for templates_joined in templates.chunks(50).map(|t| t.join("|")) {
            let params = api.params_into(&vec![
                ("action", "query"),
                ("prop", "redirects"),
                ("titles", &templates_joined),
                ("rdprop", "title"),
                ("redirects", ""),
            ]);
            
            match api.get_query_api_json_all(&params) {
                Ok(response) => {
                    if let Value::Object(map) = &response["error"] {
                        if let Some(info) = map["info"].as_str() {
                            eprintln!("error while retrieving redirects: {}", info);
                        }
                        return None;
                    } else {
                        query_responses.push(response);
                    }
                },
                Err(e) => {
                    eprintln!("error while retrieving redirects: {}", e);
                    return None;
                },
            }
        }
        
        Some(Self { query_responses })
    }
    
    pub fn template_to_redirects(&'a self) -> HashMap<&'a str, Option<Vec<&'a str>>> {
        self.query_responses
            .iter()
            .flat_map(|response| {
                response["query"][QueryKey::Pages.as_str()]
                .as_object()
                .unwrap()
                .iter()
                .map(|(_page_id, page)| {
                    let redirects = page["redirects"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .map(|c| c["title"].as_str().unwrap())
                                .collect()
                        });
                    (page["title"].as_str().unwrap(), redirects)
                })
            })
            .collect()
    }
    
    pub fn redirects_followed(&'a self) -> HashMap<&'a str, &'a str> {
        self.get_redirect_or_normalization(QueryKey::Redirects)
    }
    
    pub fn normalizations(&'a self) -> HashMap<&'a str, &'a str> {
        self.get_redirect_or_normalization(QueryKey::Normalized)
    }
    
    fn get_redirect_or_normalization(&'a self, key: QueryKey) -> HashMap<&'a str, &'a str> {
        let key = key.as_str();
        self.query_responses
            .iter()
            .filter_map(|response| {
                response["query"][key]
                    .as_array()
            })
            .flat_map(|results| {
                results.iter()
                    .map(|obj| (obj["from"].as_str().unwrap(), obj["to"].as_str().unwrap()))
            })
            .collect::<HashMap<_, _>>()
    }
}
    
// This assumes that a template and all its redirects are set to print to the same file.
pub fn add_template_redirects<V: Clone + std::fmt::Debug> (hashmap: &mut HashMap<String, V>) {
    let template_redirects = match TemplateRedirects::new(hashmap.keys()) {
        Some(t) => t,
        None => return,
    };
    let redirects_followed = template_redirects.redirects_followed();
    let normalizations = template_redirects.normalizations();
    let template_to_redirects = template_redirects.template_to_redirects();
    let mut to_insert = HashMap::new();
    for (pagename, value) in hashmap
        .iter()
        .map(|(template, value)| {
            (format!("Template:{}", template), value)
        })
    {
        let pagename = pagename.as_str();
        let normalized = match normalizations.get(pagename) {
            Some(normalized) => {
                let trimmed = normalized.trim_start_matches("Template:");
                if !hashmap.contains_key(trimmed) {
                    to_insert.insert(trimmed, (*value).clone());
                }
                normalized
            },
            None => pagename,
        };
        let redirect_target = match redirects_followed.get(normalized) {
            Some(redirect_target) => {
                let trimmed = redirect_target.trim_start_matches("Template:");
                if !hashmap.contains_key(trimmed) {
                    to_insert.insert(trimmed, (*value).clone());
                }
                redirect_target
            },
            None => normalized,
        };
        if let Some(Some(redirects)) = template_to_redirects.get(redirect_target) {
            for redirect in redirects {
                // This will not work if pagename starts in "Template:Template:",
                // but that's highly unlikely.
                let redirect = redirect.trim_start_matches("Template:");
                if !hashmap.contains_key(redirect) {
                    to_insert.insert(redirect, (*value).clone());
                }
            }
        }
    }
    
    hashmap.reserve(to_insert.len());
    for (key, value) in to_insert {
        hashmap.insert(key.to_string(), value);
    }
}

type ShareableFileWriter = Rc<RefCell<BufWriter<File>>>;

const CHAR_BEFORE_TITLE: char = '\u{1}';
const CHAR_BEFORE_TEMPLATE: char = '\n';
pub const MAX_TEMPLATE_NAME: usize = 256;

fn is_template_name_whitespace(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'_'
}

pub fn normalize_template_name<'a>(name: &str, name_buffer: &'a mut [u8]) -> Option<&'a str> {
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
            unsafe { Some(std::str::from_utf8_unchecked(name_buffer)) }
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
                        None => normalized.to_string() + ".txt",
                    };
                    Some((
                        normalized.to_string(),
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
    
    pub fn add_redirects (&mut self) {
        add_template_redirects(&mut self.template_to_file);
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
    
    fn dump (&mut self, page: &Page, template: &Node) {
        if let Template { start, end, name, .. } = template {
            let name = &name.get_text_from(&page.text);
            if name.len() <= MAX_TEMPLATE_NAME {
                let mut name_normalized = [0u8; MAX_TEMPLATE_NAME];
                let name = match normalize_template_name(name, &mut name_normalized) {
                    Some(n) => n,
                    None => return,
                };
                if let Some((file, number)) = self.template_to_file.get(name) {
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