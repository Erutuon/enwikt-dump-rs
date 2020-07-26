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
    #[serde(borrow)]
    pub name: Cow<'a, str>,
    pub parameters: BTreeMap<Cow<'a, str>, &'a str>,
}

// Avoids memory allocations for most numbered parameters.
static NUMBERS: &[&str] = &[
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13",
    "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25",
    "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37",
    "38", "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "49",
    "50", "51", "52", "53", "54", "55", "56", "57", "58", "59", "60", "61",
    "62", "63", "64", "65", "66", "67", "68", "69", "70", "71", "72", "73",
    "74", "75", "76", "77", "78", "79", "80", "81", "82", "83", "84", "85",
    "86", "87", "88", "89", "90", "91", "92", "93", "94", "95", "96", "97",
    "98", "99", "100", "101", "102", "103", "104", "105", "106", "107", "108",
    "109", "110", "111", "112", "113", "114", "115", "116", "117", "118",
    "119", "120", "121", "122", "123", "124", "125", "126", "127", "128",
    "129", "130", "131", "132", "133", "134", "135", "136", "137", "138",
    "139", "140", "141", "142", "143", "144", "145", "146", "147", "148",
    "149", "150", "151", "152", "153", "154", "155", "156", "157", "158",
    "159", "160", "161", "162", "163", "164", "165", "166", "167", "168",
    "169", "170", "171", "172", "173", "174", "175", "176", "177", "178",
    "179", "180", "181", "182", "183", "184", "185", "186", "187", "188",
    "189", "190", "191", "192", "193", "194", "195", "196", "197", "198",
    "199", "200",
];

impl<'a> TemplateBorrowed<'a> {
    pub fn new(
        wikitext: &'a str,
        name: &'a [Node<'a>],
        parameters: &'a [dump_parser::Parameter<'a>],
    ) -> Self {
        use Cow::*;
        let name = Borrowed(name.get_text_from(wikitext));
        let parameters = template_parameters::enumerate(parameters)
            .map(|(key, value)| {
                let key = match key {
                    ParameterKey::NodeList(nodes) => {
                        Borrowed(nodes.get_text_from(wikitext))
                    }
                    ParameterKey::Number(num) => {
                        if let Some(s) = NUMBERS.get(num as usize) {
                            Borrowed(*s)
                        } else {
                            Owned(num.to_string())
                        }
                    }
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

    pub fn visit<F>(&self, nodes: &'a [Node], func: &mut F)
    where
        F: FnMut(TemplateBorrowed<'a>, &'a Node),
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
                        self.wikitext,
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
#[non_exhaustive]
pub enum TitleNormalizationError {
    TooLong,
    IllegalChar,
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
pub fn normalize_title(name: &str) -> Result<String, TitleNormalizationError> {
    fn is_title_whitespace(c: char) -> bool {
        c.is_ascii_whitespace() || c == '_'
    }

    let name = name.trim_matches(is_title_whitespace);
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
