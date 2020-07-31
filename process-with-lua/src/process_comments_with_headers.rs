use dump_parser::{
    parse_wiki_text::Positioned, wiktionary_configuration, Node,
};
use rlua::{
    Context, Error as LuaError, Function, Result as LuaResult, ToLua, Value,
};
use std::collections::HashSet;
use std::convert::From;
use std::io::BufRead;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::result::Result as StdResult;
use dump_parser::Namespace;

use crate::exit_with_error;

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
pub enum VisitError {
    LuaError(LuaError),
    StopParsing,
}

impl From<LuaError> for VisitError {
    fn from(error: LuaError) -> Self {
        VisitError::LuaError(error)
    }
}

struct Visitor<'a> {
    wikitext: &'a str,
    comments: Vec<&'a str>,
    headers: HeaderStack<'a>,
}

impl<'a> Visitor<'a> {
    pub fn new(wikitext: &'a str) -> Self {
        Visitor {
            wikitext,
            comments: Vec::new(),
            headers: HeaderStack::new(),
        }
    }

    fn visit<F>(&mut self, nodes: &[Node<'a>], func: &mut F) -> LuaResult<bool>
    where
        F: FnMut(&[&'a str], &HeaderStack<'a>) -> LuaResult<bool>,
    {
        match self.do_visit(nodes, func) {
            Err(VisitError::LuaError(e)) => return Err(e),
            Err(VisitError::StopParsing) | Ok(false) => return Ok(false),
            _ => (),
        };
        // Process comments in the last section.
        if !self.comments.is_empty() {
            let result = func(&self.comments, &self.headers);
            self.comments.clear();
            result
        } else {
            Ok(true)
        }
    }

    fn do_visit<F>(
        &mut self,
        nodes: &[Node<'a>],
        func: &mut F,
    ) -> StdResult<bool, VisitError>
    where
        F: FnMut(&[&'a str], &HeaderStack<'a>) -> LuaResult<bool>,
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
                    // Process all comments under the previously encountered header
                    // (or at the beginning of the page).
                    if !self.comments.is_empty() {
                        let continue_parsing =
                            func(self.comments.as_slice(), &self.headers)?;
                        if !continue_parsing {
                            return Err(VisitError::StopParsing);
                        }
                    }
                    self.comments.clear();
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
                }
                Comment { .. } => {
                    let comment = &self.wikitext[node.start()..node.end()];
                    self.comments.push(comment);
                }
                Bold { .. }
                | BoldItalic { .. }
                | Category { .. }
                | CharacterEntity { .. }
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

pub fn process_comments_and_headers_with_function<R: BufRead>(
    dump_file: R,
    lua_func: Function,
    namespaces: HashSet<Namespace>,
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
            let continue_parsing = Visitor::new(wikitext).visit(
                &parser_output.nodes,
                &mut |comments, headers| {
                    Ok(lua_func.call((
                        comments.to_vec(),
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
