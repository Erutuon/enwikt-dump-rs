use std::collections::HashSet;
use std::convert::{From, TryInto};
use std::io::BufRead;
use std::result::Result as StdResult;
use wiktionary_namespaces::Namespace;
use parse_mediawiki_dump;
use rlua::{Context, Error as LuaError, Function, Result as LuaResult, ToLua, Value};
use dump_parser::{ wiktionary_configuration, Node };
use template_iter::parse_wiki_text_ext::get_nodes_text;

use crate::exit_with_error;

struct Header<'a> {
    text: &'a str,
    level: u8,
}

impl<'a> Header<'a> {
    fn new(text: &'a str, level: u8) -> Self {
        Header { text, level }
    }
}

impl<'lua, 'a> ToLua<'lua> for Header<'a> {
    fn to_lua(self, lua: Context<'lua>) -> LuaResult<Value<'lua>> {
        let table = lua.create_table()?;
        table.set("text", self.text)?;
        table.set("level", self.level)?;
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

struct Visitor<'a> {
    wikitext: &'a str,
}

impl<'a> Visitor<'a> {
    pub fn new(wikitext: &'a str) -> Self {
        Visitor { wikitext }
    }

    fn visit<F> (&mut self, nodes: &'a Vec<Node<'a>>, func: &mut F) -> LuaResult<bool>
        where F: FnMut(Header) -> LuaResult<bool>
    {
        match self.do_visit(nodes, func) {
            Err(VisitError::LuaError(e)) => return Err(e),
            Err(VisitError::StopParsing) | Ok(false) => return Ok(false),
            _ => (),
        };
        Ok(true)
    }

    fn do_visit<F> (&mut self, nodes: &'a Vec<Node<'a>>, func: &mut F) -> StdResult<bool, VisitError>
        where F: FnMut(Header) -> LuaResult<bool>
    {
        use dump_parser::Node::*;
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.do_visit(&item.nodes, func)?;
                    }
                },
                Heading { nodes, level, .. } => {
                    let text = get_nodes_text(self.wikitext, nodes);
                    let continue_parsing = func(Header::new(text, *level))?;
                    if !continue_parsing {
                        return Err(VisitError::StopParsing);
                    }

                    self.do_visit(&nodes, func)?;
                },
                | Preformatted { nodes, .. }
                | Tag { nodes, .. } => {
                    self.do_visit(&nodes, func)?;
                },
                Image { text, .. }
                | Link { text, .. } => {
                    self.do_visit(&text, func)?;
                },
                | OrderedList { items, .. }
                | UnorderedList { items, .. } => {
                    for item in items {
                        self.do_visit(&item.nodes, func)?;
                    }
                },
                Parameter { name, default, .. } => {
                    if let Some(nodes) = default {
                        self.do_visit(&nodes, func)?;
                    }
                    self.do_visit(&name, func)?;
                },
                Table { attributes, captions, rows, .. } => {
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
                },
                Template { name, parameters, .. } => {
                    self.do_visit(&name, func)?;
                    for parameter in parameters {
                        if let Some(name) = &parameter.name {
                            self.do_visit(name, func)?;
                        }
                        self.do_visit(&parameter.value, func)?;
                    }
                },
                | Bold {..}
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
        Ok(true)
    }
}

pub fn process_headers_with_function<R: BufRead>(
    dump_file: R,
    lua_func: Function,
    namespaces: HashSet<Namespace>,
) -> LuaResult<()> {
    let configuration = wiktionary_configuration();
    let parser = parse_mediawiki_dump::parse(dump_file)
        .map(|result| {
            result.unwrap_or_else(|e| {
                exit_with_error!("Error while parsing dump: {}", e);
            })
        });
    for page in parser {
        if namespaces.contains(&page.namespace
            .try_into()
            .unwrap_or_else(|_| {
                exit_with_error!("unrecognized namespace number {}", page.namespace);
            })
        ) {
            let wikitext = &page.text;
            let parser_output = configuration.parse(&page.text);
            let continue_parsing = Visitor::new(wikitext)
                .visit(&parser_output.nodes, &mut |header| {
                    /*
                    for template in templates {
                        use template_dumper::{MAX_TEMPLATE_NAME, normalize_template_name};
                        let mut normalized_name = [0u8; MAX_TEMPLATE_NAME];
                        normalize_template_name(template.name, &mut normalized_name);
                    }
                    */
                    Ok(lua_func.call(
                        (
                            header,
                            page.title.as_str()
                        )
                    )?)
                })?;
            if !continue_parsing {
                break;
            }
        }
    }

    Ok(())
}
