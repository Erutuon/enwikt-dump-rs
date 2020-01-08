use dump_parser::{self, Node, Parameter};
use parse_wiki_text::Positioned;
use parse_wiki_text_ext::template_parameters::{self, ParameterKey};
use serde::{Deserialize, Serialize};
use serde_cbor::{self, Deserializer as CborDeserializer};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    convert::TryInto,
    fs::File,
    io::BufReader,
    path::Path,
};
use structopt::StructOpt;
use wiktionary_namespaces::Namespace;

#[derive(Debug, Serialize, Deserialize)]
struct Template<'a> {
    name: &'a str,
    parameters: BTreeMap<Cow<'a, str>, &'a str>,
}

impl<'a> Template<'a> {
    pub fn new(
        wikitext: &'a str,
        name: &'a Vec<Node<'a>>,
        parameters: &'a Vec<Parameter<'a>>,
    ) -> Self {
        let name = &name.get_text_from(wikitext);
        let parameters = template_parameters::enumerate(parameters)
            .map(|(key, value)| {
                let key = match key {
                    ParameterKey::NodeList(nodes) => {
                        Cow::Borrowed(&nodes.get_text_from(wikitext))
                    }
                    ParameterKey::Number(num) => Cow::Owned(num.to_string()),
                };
                (key, &value.get_text_from(wikitext))
            })
            .collect();
        Self { name, parameters }
    }

    #[allow(dead_code)]
    pub fn from_node(
        wikitext: &'a str,
        template: &'a Node<'a>,
    ) -> Result<Self, &'static str> {
        if let Node::Template {
            name, parameters, ..
        } = template
        {
            Ok(Template::new(wikitext, name, parameters))
        } else {
            Err("not a template")
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateOwned {
    name: String,
    parameters: BTreeMap<String, String>,
}

impl<'a> From<Template<'a>> for TemplateOwned {
    fn from(template: Template) -> Self {
        let name = template.name.into();
        let parameters = template
            .parameters
            .iter()
            .map(|(key, value)| {
                (key.to_owned().into(), value.to_owned().into())
            })
            .collect();
        Self { name, parameters }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplatesInPage {
    title: String,
    templates: Vec<TemplateOwned>,
}

fn print_cbor_from_path<P: AsRef<Path>>(path: P) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for template in
        CborDeserializer::from_reader(reader).into_iter::<TemplatesInPage>()
    {
        println!("{:?}", template.unwrap());
    }
}

struct TemplateVisitor<'a> {
    wikitext: &'a str,
}

impl<'a> TemplateVisitor<'a> {
    pub fn new(wikitext: &'a str) -> Self {
        TemplateVisitor { wikitext }
    }

    pub fn visit<F>(&self, nodes: &Vec<Node>, func: &mut F)
    where
        F: FnMut(&Self, &Node),
    {
        use parse_wiki_text::Node::*;

        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.visit(&item.nodes, func);
                    }
                }
                Heading { nodes, .. }
                | Preformatted { nodes, .. }
                | Tag { nodes, .. } => {
                    self.visit(&nodes, func);
                }
                Image { text, .. } | Link { text, .. } => {
                    self.visit(&text, func);
                }
                OrderedList { items, .. } | UnorderedList { items, .. } => {
                    for item in items {
                        self.visit(&item.nodes, func);
                    }
                }
                Parameter { name, default, .. } => {
                    match default {
                        Some(nodes) => self.visit(&nodes, func),
                        None => {}
                    }
                    self.visit(&name, func);
                }
                Table {
                    attributes,
                    captions,
                    rows,
                    ..
                } => {
                    self.visit(&attributes, func);
                    for caption in captions {
                        if let Some(attributes) = &caption.attributes {
                            self.visit(attributes, func)
                        }
                        self.visit(&caption.content, func);
                    }
                    for row in rows {
                        self.visit(&row.attributes, func);
                        for cell in &row.cells {
                            if let Some(attributes) = &cell.attributes {
                                self.visit(attributes, func);
                            }
                            self.visit(&cell.content, func);
                        }
                    }
                }
                Template {
                    name, parameters, ..
                } => {
                    self.visit(&name, func);
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.visit(name, func);
                        }
                        self.visit(&parameter.value, func);
                    }
                    func(&self, &node);
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
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opts {
    #[structopt(
        long = "namespace",
        short,
        value_delimiter = ",",
        default_value = "main"
    )]
    /// namespace to process
    namespaces: Vec<Namespace>,
    #[structopt(short, long)]
    /// number of pages to process [default: unlimited]
    pages: Option<usize>,
    /// path to pages-articles.xml or pages-meta-current.xml
    #[structopt(
        long = "input",
        short = "i",
        default_value = "pages-articles.xml"
    )]
    dump_filepath: String,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (first, second) = (args.next(), args.next());
    if first.as_ref().map_or(false, |s| s == "read") {
        let path = second.expect("provide path to read from");
        print_cbor_from_path(path);
    } else {
        let opts = Opts::from_args();
        use dump_parser::wiktionary_configuration as create_configuration;
        let file =
            File::open(opts.dump_filepath).expect("Wiktionary dump file");
        let configuration = create_configuration();
        let namespaces: HashSet<_> = opts.namespaces.into_iter().collect();
        let parser = dump_parser::parse(file)
            .map(|result| {
                result.unwrap_or_else(|e| {
                    panic!("Error while parsing dump: {}", e);
                })
            })
            .filter(|page| {
                namespaces.contains(&page.namespace.try_into().unwrap())
            })
            .take(opts.pages.unwrap_or(std::usize::MAX));
        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        for page in parser {
            let wikitext = &page.text;
            let output = configuration.parse(wikitext);
            let mut templates: Option<Vec<TemplateOwned>> = None;
            TemplateVisitor::new(wikitext).visit(
                &output.nodes,
                &mut |visitor, node| {
                    if let Node::Template {
                        name, parameters, ..
                    } = node
                    {
                        let parsed_template = Template::new(
                            &visitor.wikitext,
                            &name,
                            &parameters,
                        );
                        if parsed_template.name == "m" {
                            if templates.is_none() {
                                templates = Some(Vec::new())
                            }
                            if let Some(templates) = &mut templates {
                                templates.push(parsed_template.into());
                            }
                        }
                    }
                },
            );
            if templates.is_some() {
                serde_cbor::to_writer(
                    &mut writer,
                    &TemplatesInPage {
                        title: page.title.to_string(),
                        templates: templates.unwrap(),
                    },
                )
                .unwrap();
            }
        }
    }
}
