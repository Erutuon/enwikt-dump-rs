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

fn trim_template_prefix<'a>(pagename: &'a str) -> &'a str {
    if pagename.starts_with("Template:") {
        &pagename["Template:".len()..]
    } else {
        pagename
    }
}

struct TemplateToVal<V>(HashMap<String, V>);

impl<V: Clone + std::fmt::Debug> TemplateToVal<V> {
    fn new() -> Self { Self(HashMap::new()) }
    
    fn insert(&mut self, key: &str, value: &V) {
        if !self.0.contains_key(key) {
            self.0.insert(key.to_string(), (*value).clone());
        }
    }
    
    fn into_inner(self) -> HashMap<String, V> {
        self.0
    }
}
    
// This assumes that a template and all its redirects are set to print to the same file.
// Should probably return Result.
pub fn add_template_redirects<V: Clone + std::fmt::Debug> (
    hashmap: &HashMap<String, V>
) -> Option<HashMap<String, V>> {
    let template_redirects = match TemplateRedirects::new(hashmap.keys()) {
        Some(t) => t,
        None => return None,
    };
    let redirects_followed = template_redirects.redirects_followed();
    let normalizations = template_redirects.normalizations();
    let template_to_redirects = template_redirects.template_to_redirects();
    let mut new_hashmap = TemplateToVal::new();
    // Let value for main page take precedence over value for redirects to that page.
    for (pagename, redirects) in &template_to_redirects {
        let main_template = trim_template_prefix(pagename);
        if let Some(value) = hashmap.get(main_template) {
            new_hashmap.insert(main_template, value);
            if let Some(redirects) = redirects {
                for redirect in redirects {
                    new_hashmap.insert(trim_template_prefix(redirect), value);
                }
            }
        }
    }
    for (pagename, value) in hashmap
        .iter()
        .map(|(template, value)| {
            (format!("Template:{}", template), value)
        })
    {
        let mut pagename = pagename.as_str();
        if let Some(normalized) = normalizations.get(pagename) {
            pagename = normalized;
        }
        if let Some(redirect_target) = redirects_followed.get(pagename) {
            pagename = redirect_target;
        }
        new_hashmap.insert(trim_template_prefix(pagename), value);
        if let Some(Some(redirects)) = template_to_redirects.get(pagename) {
            for redirect in redirects.iter() {
                new_hashmap.insert(trim_template_prefix(redirect), value);
            }
        }
    }
    
    Some(new_hashmap.into_inner())
}

type ShareableFileWriter = Rc<RefCell<BufWriter<File>>>;

const CHAR_BEFORE_TITLE: char = '\u{1}';
const CHAR_BEFORE_TEMPLATE: char = '\n';

#[derive(Debug, PartialEq, Eq)]
pub enum TitleNormalizationError {
    TooLong,
    IllegalChar,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub const TITLE_MAX: usize = 255;

// This trims whitespace on either end and converts
// sequences of whitespace characters to a single underscore.
// In titles, underscores count as whitespace.
// This is only a subset of the stuff done when resolving a template name.
// TODO: figure out all the characters that count as whitespace
// at the beginning and end and when collapsing whitespace sequences,
// and all the illegal characters. Perhaps this information is in one of the
// routines called by Title.php?
// Perhaps also decode HTML character entities?
pub fn normalize_title<'a>(name: &str) -> Result<String, TitleNormalizationError> {
    fn is_title_whitespace(c: char) -> bool {
        c.is_ascii_whitespace() || c == '_'
    }
    
    let name = name.trim_matches(|c| is_title_whitespace(c));
    let mut normalized_title = String::new();
    let mut name_iter = name.chars().peekable();
    while let Some(c) = name_iter.next() {
        if normalized_title.len() >= TITLE_MAX {
            return Err(TitleNormalizationError::TooLong);
        }
        if ('\u{00}'..'\u{1F}').contains(&c) {
            return Err(TitleNormalizationError::IllegalChar);
        }
        if is_title_whitespace(c) {
            normalized_title.push('_');
            while name_iter.peek().map(|c| is_title_whitespace(*c)) == Some(true) {
                let _ = name_iter.next();
            }
        } else {
            normalized_title.push(c);
        }
    }
    if normalized_title.len() > TITLE_MAX {
        Err(TitleNormalizationError::TooLong)
    } else {
        Ok(normalized_title)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_normalize_title() {
        use super::{normalize_title, TitleNormalizationError, TITLE_MAX};
        use std::iter;
        
        for (name, normalized) in &[
            (iter::repeat('_').take(TITLE_MAX)
                .chain(iter::once('l')
                .chain(iter::repeat(' ').take(TITLE_MAX))).collect(), Ok("l".to_string())),
            (iter::repeat("_").take(TITLE_MAX)
                .chain(iter::once("auto")
                .chain(iter::repeat(" ").take(TITLE_MAX)))
                .chain(iter::once("cat")
                .chain(iter::repeat(" ").take(TITLE_MAX))).collect(), Ok("auto_cat".to_string())),
            (iter::repeat('a').take(TITLE_MAX).collect(), Ok(iter::repeat('a').take(TITLE_MAX).collect())),
            (iter::repeat('a').take(TITLE_MAX + 1).collect(), Err(TitleNormalizationError::TooLong)),
            ("\u{0}".to_string(), Err(TitleNormalizationError::IllegalChar)),
        ] {
            assert_eq!(&normalize_title(name), normalized);
        }
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
                let normalized = normalize_title(&template).ok()?;
                let filepath = match filepath {
                    Some(p) => p.to_string(),
                    None => normalized.clone() + ".txt",
                };
                Some((
                    normalized,
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
            })
            .collect();
        
        let title_printed = vec![false; file_number];
        
        Self { template_to_file, title_printed }
    }
    
    pub fn add_redirects (&mut self) {
        if let Some(map) = add_template_redirects(&self.template_to_file) {
            self.template_to_file = map;
        }
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
            if let Ok(name) = normalize_title(&name.get_text_from(&page.text)) {
                if let Some((file, number)) = self.template_to_file.get(&name) {
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