use std::{
    borrow::Cow,
    collections::BTreeMap,
};
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use parse_wiki_text_ext::{
    get_nodes_text,
    template_parameters::{self, ParameterKey},
};
use wiktionary_namespaces::Namespace;
use dump_parser::{
    self,
    Node::{self, *},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateBorrowed<'a> {
    pub name: &'a str,
    pub parameters: BTreeMap<Cow<'a, str>, &'a str>,
}

impl<'a> TemplateBorrowed<'a> {
    pub fn new (
        wikitext: &'a str,
        name: &'a Vec<Node<'a>>,
        parameters: &'a Vec<dump_parser::Parameter<'a>>
    ) -> Self {
        let name = get_nodes_text(wikitext, &name);
        let parameters = template_parameters::enumerate(parameters)
            .map(|(key, value)| {
                let key = match key {
                    ParameterKey::NodeList(nodes) => {
                        Cow::Borrowed(get_nodes_text(wikitext, &nodes))
                    },
                    ParameterKey::Number(num) => {
                        Cow::Owned(num.to_string())
                    },
                };
                (key, get_nodes_text(wikitext, &value))
            })
            .collect();
        Self { name, parameters }
    }
    
    #[allow(dead_code)]
    pub fn from_node(
        wikitext: &'a str,
        template: &'a Node<'a>
    ) -> Result<Self, &'static str> {
        if let Template { name, parameters, .. } = template {
            Ok(TemplateBorrowed::new(wikitext, name, parameters))
        } else {
            Err("not a template")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateOwned {
    pub name: String,
    pub parameters: BTreeMap<String, String>,
}

impl<'a> From<TemplateBorrowed<'a>> for TemplateOwned {
    fn from(template: TemplateBorrowed) -> Self {
        let name = template.name.into();
        let parameters = template.parameters
            .iter()
            .map(|(key, value)| {
                (key.to_owned().into(), value.to_owned().into())
            })
            .collect();
        Self { name, parameters }
    }
}

pub struct TemplateVisitor<'a> {
    wikitext: &'a str,
}

impl<'a> TemplateVisitor<'a> {
    pub fn new(
        wikitext: &'a str,
    ) -> Self {
        TemplateVisitor { wikitext }
    }
    
    pub fn visit<F> (&self, nodes: &Vec<Node>, func: &mut F)
        where F: FnMut(TemplateBorrowed, &Node)
    {
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.visit(&item.nodes, func);
                    }
                },
                  Heading { nodes, .. }
                | Preformatted { nodes, .. }
                | Tag { nodes, .. } => {
                    self.visit(&nodes, func);
                },
                  Image { text, .. }
                | Link { text, .. } => {
                    self.visit(&text, func);
                },
                  OrderedList { items, .. }
                | UnorderedList { items, .. } => {
                    for item in items {
                        self.visit(&item.nodes, func);
                    }
                },
                Parameter { name, default, .. } => {
                    if let Some(nodes) = default {
                        self.visit(&nodes, func);
                    }
                    self.visit(&name, func);
                },
                Table { attributes, captions, rows, .. } => {
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
                },
                Template { name, parameters, .. } => {
                    self.visit(&name, func);
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.visit(name, func);
                        }
                        self.visit(&parameter.value, func);
                    }
                    let template = TemplateBorrowed::new(&self.wikitext, &name, &parameters);
                    func(template, &node);
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
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opts {
    #[structopt(
        long = "namespace",
        short,
        value_delimiter = ",",
        default_value = "main",
    )]
    /// namespace to process
    namespaces: Vec<Namespace>,
    #[structopt(short, long)]
    /// number of pages to process [default: unlimited]
    pages: Option<usize>,
    /// path to pages-articles.xml or pages-meta-current.xml
    #[structopt(long = "input", short = "i", default_value = "pages-articles.xml")]
    dump_filepath: String,
}

/*
use serde_cbor;
use std::{
    collections::HashSet,
    convert::TryInto,
    fs::File,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplatesInPage {
    pub title: String,
    pub templates: Vec<TemplateOwned>,
}
fn main() {
    let opts = Opts::from_args();
    use dump_parser::wiktionary_configuration as create_configuration;
    let file = File::open(opts.dump_filepath).expect("Wiktionary dump file");
    let configuration = create_configuration();
    let namespaces: HashSet<_> = opts.namespaces
        .into_iter()
        .collect();
    let parser = dump_parser::parse(file)
        .map(|result| {
            result.unwrap_or_else(|e| {
                panic!("Error while parsing dump: {}", e);
            })
        })
        .filter(|page| namespaces.contains(&page.namespace.try_into().unwrap()))
        .take(opts.pages.unwrap_or(std::usize::MAX));
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    for page in parser {
        let wikitext = &page.text;
        let output = configuration.parse(wikitext);
        let mut templates: Vec<TemplateOwned> = Vec::new();
        TemplateVisitor::new(wikitext).visit(&output.nodes, &mut |_wikitext, template| {
            if template.name == "m" {
                templates.push(template.into());
            }
        });
        if templates.len() > 0 {
            let title = page.title.to_string();
            serde_cbor::to_writer(
                &mut writer,
                &TemplatesInPage { title, templates }
            ).unwrap();
        }
    }
}
*/