use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    cell::RefCell,
    fmt::Write as WriteFmt,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    rc::Rc,
    str::FromStr,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use parse_mediawiki_dump::Page;
use parse_wiki_text::{self, Node::{self, *}, Warning};

mod configuration;
use configuration::create as create_configuration;

mod namespace;
use namespace::Namespace;

mod nodes_ext;
use nodes_ext::get_nodes_text;

// mod template_parameters;

const CHAR_BEFORE_TITLE: char = '\u{1}';
const CHAR_BEFORE_TEMPLATE: char = '\n';
const MAX_TEMPLATE_NAME: usize = 256;

type DumpParser = parse_mediawiki_dump::Parser<BufReader<File>>;
type ShareableFileWriter = Rc<RefCell<BufWriter<File>>>;

fn parse_wiktionary_dump (dump_file: File) -> DumpParser {
    let reader = BufReader::new(dump_file);
    parse_mediawiki_dump::parse(reader)
}

fn is_template_name_whitespace(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'_'
}

pub fn normalize_template_name<'a>(name: &str, name_buffer: &'a mut [u8]) -> Option<&'a [u8]> {
    match name.bytes()
        .position(|b| !is_template_name_whitespace(b)) {
        Some(start_index) => {
            // This can't fail because finding the start index proves
            // there's a non-whitespace character in the template name.
            let end_index = name.bytes().len() - name.bytes()
                .rev()
                .position(|b| !is_template_name_whitespace(b))
                .unwrap();
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
struct TemplateDumper {
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
        namespaces: Vec<Namespace>
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

fn parse_namespace (namespace: &str) -> Result<u32, &str> {
    if let Ok(n) =  u32::from_str(namespace) {
        Ok(n)
    } else {
        namespace.parse::<Namespace>().map(u32::from)
    }
}

#[derive(StructOpt, Debug, Clone)]
struct Args {
    #[structopt(long = "templates", short)]
    /// paths of files containing template names with optional tab and output filepath
    template_filepaths: Vec<String>,
    #[structopt(
        long = "namespace",
        short,
        parse(try_from_str = "parse_namespace"),
        value_delimiter = ",",
    )]
    /// namespace to process [default: 0 (main)]
    namespaces: Vec<u32>,
    #[structopt(short, long)]
    /// number of pages to process [default: no limit]
    limit: Option<usize>,
    /// path to pages-articles.xml or pages-meta-current.xml
    #[structopt(long = "input", short = "i", default_value = "pages-articles.xml")]
    dump_filepath: String,
}

#[derive(Debug)]
struct Opts {
    limit: usize,
    files: Vec<(String, Option<String>)>,
    namespaces: Vec<Namespace>,
    dump_file: File,
    // files: HashMap<PathBuf, File>,
}

fn collect_template_names_and_files(template_filepaths: Vec<String>)
    -> Vec<(String, Option<String>)>
{
    let mut template_and_file: Vec<(String, Option<String>)> = Vec::new();
    for template_filepath in template_filepaths {
        let file = File::open(&template_filepath).unwrap_or_else(|e| {
            panic!("could not open file {}: {}", template_filepath, e);
        });
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line.unwrap_or_else(|e| {
                panic!("error while reading line {} in {}: {}",
                    i, template_filepath, e);
            });
            let mut iter = line.splitn(2, '\t');
            let template = iter.next().unwrap_or_else(|| {
                panic!("could not split line {} in {}",
                    i, template_filepath);
            }).to_string();
            let filepath = iter.next().map(ToString::to_string);
            template_and_file.push((template, filepath));
        }
    }
    template_and_file
}

fn get_opts() -> Opts {
    let args = Args::from_args();
    let Args { template_filepaths, namespaces, limit, dump_filepath } = args;
    let mut namespaces: Vec<Namespace> = namespaces.iter()
        .map(|n| Namespace::try_from(*n).unwrap_or_else(|_| {
            panic!("{} is not a valid namespace id", n)
        }))
        .collect();
    if dbg!(&namespaces).is_empty() {
        namespaces.push(Namespace::Main);
    }
    let limit = limit.unwrap_or(std::usize::MAX);
    let files = collect_template_names_and_files(template_filepaths);
    let dump_file = File::open(dump_filepath).unwrap_or_else(|e|
        panic!("did not find pages-articles.xml: {}", e)
    );
    Opts { limit, namespaces, files, dump_file }
}

fn print_time(time: &Duration) -> String {
    let nanos = time.subsec_nanos();
    let mut secs = time.as_secs();
    let minutes = secs / 60;
    let mut printed = String::new();
    if minutes > 0 {
        secs = secs % 60;
        write!(printed, "{}m ", minutes).unwrap();
    }
    write!(printed, "{}.", secs).unwrap();
    let decimals = format!("{:09}", nanos);
    printed.push_str({
        if secs == 0 {
            let zero_count = decimals.as_bytes().iter()
                .take_while(|&&b| b == b'0')
                .count();
            match zero_count {
                0...2 => &decimals[..3],
                3...5 => &decimals[..6],
                _     => &decimals[..9],
            }
        } else {
            &decimals[..3]
        }
    });
    write!(printed, "s").unwrap();
    printed
}

fn main() {
    let main_start = Instant::now();
    let opts = get_opts();
    let parser = parse_wiktionary_dump(opts.dump_file);
    let mut dumper = TemplateDumper::new(opts.files);
    let start_time = main_start.elapsed();
    let parse_start = Instant::now();
    dumper.parse(parser, opts.limit, opts.namespaces);
    let parse_time = parse_start.elapsed();
    eprintln!("startup took {}, parsing {}",
        print_time(&start_time),
        print_time(&parse_time));
}