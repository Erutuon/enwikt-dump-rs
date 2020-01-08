use dump_parser::{
    self,
    Node::{self, *},
    Positioned,
};
pub use parse_wiki_text_ext;
use parse_wiki_text_ext::template_parameters::{self, ParameterKey};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateBorrowed<'a> {
    pub name: &'a str,
    pub parameters: BTreeMap<Cow<'a, str>, &'a str>,
}

impl<'a> TemplateBorrowed<'a> {
    pub fn new(
        wikitext: &'a str,
        name: &'a Vec<Node<'a>>,
        parameters: &'a Vec<dump_parser::Parameter<'a>>,
    ) -> Self {
        let name = &name.get_text_from(wikitext);
        let parameters = template_parameters::enumerate(parameters)
            .map(|(key, value)| {
                let key = match key {
                    ParameterKey::NodeList(nodes) => {
                        Cow::Borrowed(nodes.get_text_from(wikitext))
                    }
                    ParameterKey::Number(num) => Cow::Owned(num.to_string()),
                };
                (key, value.get_text_from(wikitext))
            })
            .collect();
        Self { name, parameters }
    }

    #[allow(dead_code)]
    pub fn from_node(
        wikitext: &'a str,
        template: &'a Node<'a>,
    ) -> Result<Self, &'static str> {
        if let Template {
            name, parameters, ..
        } = template
        {
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

pub struct TemplateVisitor<'a> {
    wikitext: &'a str,
}

impl<'a> TemplateVisitor<'a> {
    pub fn new(wikitext: &'a str) -> Self {
        TemplateVisitor { wikitext }
    }

    pub fn visit<F>(&self, nodes: &Vec<Node>, func: &mut F)
    where
        F: FnMut(TemplateBorrowed, &Node),
    {
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
                    if let Some(nodes) = default {
                        self.visit(&nodes, func);
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
                    let template = TemplateBorrowed::new(
                        &self.wikitext,
                        &name,
                        &parameters,
                    );
                    func(template, &node);
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
pub fn normalize_title<'a>(
    name: &str,
) -> Result<String, TitleNormalizationError> {
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
            while name_iter.peek().map(|c| is_title_whitespace(*c))
                == Some(true)
            {
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
        use super::{normalize_title, TitleNormalizationError::*, TITLE_MAX};
        use std::iter;

        fn rep<T: Clone>(c: T, n: usize) -> iter::Take<iter::Repeat<T>> {
            iter::repeat(c).take(n)
        }

        for (name, normalized) in &[
            (
                rep('_', TITLE_MAX)
                    .chain(iter::once('l').chain(rep(' ', TITLE_MAX)))
                    .collect(),
                Ok("l".to_string()),
            ),
            (
                rep("_", TITLE_MAX)
                    .chain(iter::once("auto").chain(rep(" ", TITLE_MAX)))
                    .chain(iter::once("cat").chain(rep(" ", TITLE_MAX)))
                    .collect(),
                Ok("auto_cat".to_string()),
            ),
            (
                rep('a', TITLE_MAX).collect(),
                Ok(rep('a', TITLE_MAX).collect()),
            ),
            (
                rep('a', TITLE_MAX).chain(iter::once(' ')).collect(),
                Ok(rep('a', TITLE_MAX).collect()),
            ),
            (rep('a', TITLE_MAX + 1).collect(), Err(TooLong)),
            ("\u{0}".to_string(), Err(IllegalChar)),
        ] {
            assert_eq!(&normalize_title(name), normalized);
        }
    }
}
