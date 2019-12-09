use std::{
    borrow::Cow,
    collections::BTreeMap,
};
use serde::{Serialize, Deserialize};
pub use parse_wiki_text_ext;
use parse_wiki_text_ext::template_parameters::{self, ParameterKey};
use dump_parser::{
    self,
    Node::{self, *},
    Positioned,
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
        let name = &name.get_text_from(wikitext);
        let parameters = template_parameters::enumerate(parameters)
            .map(|(key, value)| {
                let key = match key {
                    ParameterKey::NodeList(nodes) => {
                        Cow::Borrowed(nodes.get_text_from(wikitext))
                    },
                    ParameterKey::Number(num) => {
                        Cow::Owned(num.to_string())
                    },
                };
                (key, value.get_text_from(wikitext))
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