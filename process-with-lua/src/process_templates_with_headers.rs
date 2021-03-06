use dump_parser::{
    parse_wiki_text::Positioned, wiktionary_configuration, Node,
};
use rlua::{
    Context, Error as LuaError, Function, Result as LuaResult, ToLua, Value,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::convert::From;
use std::io::BufRead;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::result::Result as StdResult;
use std::string::ToString;
use string_wrapper::StringWrapper;
use template_iter::{
    normalize_title,
    parse_wiki_text_ext::template_parameters::{self, ParameterKey},
};
use dump_parser::Namespace;

use crate::exit_with_error;

#[derive(Debug)]
pub struct BorrowedTemplateWithText<'a> {
    // This should be `[u8; 255]`, that is `[u8; TITLE_MAX]`,
    // but the crate does not implement it for 2^n - 1.
    name: StringWrapper<[u8; 256]>,
    parameters: BTreeMap<Cow<'a, str>, &'a str>,
    text: &'a str,
}

impl<'a> BorrowedTemplateWithText<'a> {
    pub fn new(
        wikitext: &'a str,
        name: &'a str,
        parameters: &'a [dump_parser::Parameter<'a>],
        template: &'a Node,
    ) -> StdResult<Self, &'static str> {
        let name = if let Ok(Some(name)) =
            normalize_title(name).map(|n| StringWrapper::from_str_safe(&n))
        {
            name
        } else {
            return Err("invalid template name");
        };
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
        let text = &wikitext[template.start()..template.end()];
        Ok(Self {
            name,
            parameters,
            text,
        })
    }
}

impl<'lua, 'a> ToLua<'lua> for &'a BorrowedTemplateWithText<'_> {
    fn to_lua(self, lua: Context<'lua>) -> LuaResult<Value<'lua>> {
        let table = lua.create_table()?;
        table.set("name", &*self.name)?;
        let parameters = lua.create_table_from(
            self.parameters.iter().map(|(k, v)| (k.to_string(), *v)),
        )?;
        table.set("parameters", parameters)?;
        table.set("text", self.text)?;
        Ok(Value::Table(table))
    }
}

struct SliceOfBorrowedTemplateWithText<'a, 'b>(
    &'a [BorrowedTemplateWithText<'b>],
);

impl<'lua, 'a, 'b> ToLua<'lua> for SliceOfBorrowedTemplateWithText<'a, 'b> {
    fn to_lua(self, lua: Context<'lua>) -> LuaResult<Value<'lua>> {
        let sequence = lua.create_sequence_from(self.0)?;
        Ok(Value::Table(sequence))
    }
}

const LOWEST_HEADER: usize = 1;
const HIGHEST_HEADER: usize = 6;
struct HeaderStack<'a>([Option<&'a str>; HIGHEST_HEADER]);

impl<'a> HeaderStack<'a> {
    fn new() -> Self {
        HeaderStack([None; HIGHEST_HEADER])
    }
}

impl<'a> Deref for HeaderStack<'a> {
    type Target = [Option<&'a str>; HIGHEST_HEADER];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for HeaderStack<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Index<usize> for HeaderStack<'a> {
    type Output = Option<&'a str>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index - LOWEST_HEADER]
    }
}

impl<'a> IndexMut<usize> for HeaderStack<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index - LOWEST_HEADER]
    }
}

impl<'a, 'b, 'lua> ToLua<'lua> for &'a HeaderStack<'b> {
    fn to_lua(self, lua: Context<'lua>) -> LuaResult<Value<'lua>> {
        let table = lua.create_table()?;
        for i in LOWEST_HEADER..HIGHEST_HEADER {
            table.set(i, self[i])?;
        }
        Ok(Value::Table(table))
    }
}

// This error type is solely to make it easier to exit from Visitor::do_visit.
// Also used by process_templates.
pub enum VisitError {
    LuaError(LuaError),
    StopParsing,
}

impl From<LuaError> for VisitError {
    fn from(error: LuaError) -> Self {
        VisitError::LuaError(error)
    }
}

struct Visitor<'a, 'b> {
    wikitext: &'a str,
    templates: Vec<BorrowedTemplateWithText<'a>>,
    headers: HeaderStack<'a>,
    template_filter: &'b HashSet<String>,
}

impl<'a, 'b> Visitor<'a, 'b> {
    pub fn new(
        wikitext: &'a str,
        template_filter: &'b HashSet<String>,
    ) -> Self {
        Visitor {
            wikitext,
            templates: Vec::new(),
            headers: HeaderStack::new(),
            template_filter,
        }
    }

    fn visit<F>(
        &mut self,
        nodes: &'a [Node<'a>],
        func: &mut F,
    ) -> LuaResult<bool>
    where
        F: FnMut(
            &[BorrowedTemplateWithText],
            &HeaderStack<'a>,
        ) -> LuaResult<bool>,
    {
        match self.do_visit(nodes, func) {
            Err(VisitError::LuaError(e)) => return Err(e),
            Err(VisitError::StopParsing) | Ok(false) => return Ok(false),
            _ => (),
        };
        // Process templates in the last section.
        if !self.templates.is_empty() {
            let result = func(self.templates.as_slice(), &self.headers);
            self.templates.clear();
            result
        } else {
            Ok(true)
        }
    }

    fn do_visit<F>(
        &mut self,
        nodes: &'a [Node<'a>],
        func: &mut F,
    ) -> StdResult<bool, VisitError>
    where
        F: FnMut(
            &[BorrowedTemplateWithText],
            &HeaderStack<'a>,
        ) -> LuaResult<bool>,
    {
        use dump_parser::Node::*;
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.do_visit(&item.nodes, func)?;
                    }
                }
                Heading { nodes, level, .. } => {
                    // Process all templates under the previously encountered header
                    // (or at the beginning of the page).
                    if !self.templates.is_empty() {
                        let continue_parsing =
                            func(self.templates.as_slice(), &self.headers)?;
                        if !continue_parsing {
                            return Err(VisitError::StopParsing);
                        }
                    }
                    self.templates.clear();
                    let level = *level as usize;
                    self.headers[level] =
                        Some(&nodes.get_text_from(&self.wikitext));
                    for i in level + 1..HIGHEST_HEADER {
                        self.headers[i] = None;
                    }

                    self.do_visit(&nodes, func)?;
                }
                Preformatted { nodes, .. } | Tag { nodes, .. } => {
                    self.do_visit(&nodes, func)?;
                }
                Image { text, .. } | Link { text, .. } => {
                    self.do_visit(&text, func)?;
                }
                OrderedList { items, .. } | UnorderedList { items, .. } => {
                    for item in items {
                        self.do_visit(&item.nodes, func)?;
                    }
                }
                Parameter { name, default, .. } => {
                    if let Some(nodes) = default {
                        self.do_visit(&nodes, func)?;
                    }
                    self.do_visit(&name, func)?;
                }
                Table {
                    attributes,
                    captions,
                    rows,
                    ..
                } => {
                    self.do_visit(&attributes, func)?;
                    for caption in captions {
                        if let Some(attributes) = &caption.attributes {
                            self.do_visit(attributes, func)?;
                        }
                        self.do_visit(&caption.content, func)?;
                    }
                    for row in rows {
                        self.do_visit(&row.attributes, func)?;
                        for cell in &row.cells {
                            if let Some(attributes) = &cell.attributes {
                                self.do_visit(attributes, func)?;
                            }
                            self.do_visit(&cell.content, func)?;
                        }
                    }
                }
                Template {
                    name, parameters, ..
                } => {
                    self.do_visit(&name, func)?;
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.do_visit(name, func)?;
                        }
                        self.do_visit(&parameter.value, func)?;
                    }
                    let name = name.get_text_from(&self.wikitext);
                    if self.template_filter.contains(name) {
                        if let Ok(template) = BorrowedTemplateWithText::new(
                            &self.wikitext,
                            &name,
                            &parameters,
                            &node,
                        ) {
                            self.templates.push(template);
                        }
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
        Ok(true)
    }
}

pub fn process_templates_and_headers_with_function<R: BufRead>(
    dump_file: R,
    lua_func: Function,
    namespaces: HashSet<Namespace>,
    templates: HashSet<String>,
) -> LuaResult<()> {
    let configuration = wiktionary_configuration();
    let parser = dump_parser::parse(dump_file).map(|result| {
        result.unwrap_or_else(|e| {
            exit_with_error!("Error while parsing dump: {}", e);
        })
    });
    for page in parser {
        if namespaces.contains(&page.namespace) {
            let wikitext = &page.text;
            let parser_output = configuration.parse(&page.text);
            let continue_parsing = Visitor::new(wikitext, &templates).visit(
                &parser_output.nodes,
                &mut |templates, headers| {
                    Ok(lua_func.call((
                        SliceOfBorrowedTemplateWithText(&templates),
                        headers,
                        page.title.as_str(),
                    ))?)
                },
            )?;
            if !continue_parsing {
                break;
            }
        }
    }

    Ok(())
}
