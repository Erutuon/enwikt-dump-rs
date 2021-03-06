use dump_parser::{wiktionary_configuration, Node, Positioned};
use rlua::{Function, Result as LuaResult};
use std::{collections::HashSet, io::BufRead, result::Result as StdResult};
use dump_parser::Namespace;

use crate::process_templates_with_headers::{
    BorrowedTemplateWithText, VisitError,
};

pub struct TemplateVisitor<'a, 'b> {
    wikitext: &'a str,
    template_filter: &'b HashSet<String>,
}

impl<'a, 'b> TemplateVisitor<'a, 'b> {
    pub fn new(
        wikitext: &'a str,
        template_filter: &'b HashSet<String>,
    ) -> Self {
        TemplateVisitor {
            wikitext,
            template_filter,
        }
    }

    fn visit<F>(&mut self, nodes: &[Node<'a>], func: &mut F) -> LuaResult<bool>
    where
        F: FnMut(BorrowedTemplateWithText) -> LuaResult<bool>,
    {
        match self.do_visit(nodes, func) {
            Err(VisitError::LuaError(e)) => Err(e),
            Err(VisitError::StopParsing) | Ok(false) => Ok(false),
            Ok(true) => Ok(true),
        }
    }

    pub fn do_visit<F>(
        &self,
        nodes: &[Node<'a>],
        func: &mut F,
    ) -> StdResult<bool, VisitError>
    where
        F: FnMut(BorrowedTemplateWithText) -> LuaResult<bool>,
    {
        use dump_parser::Node::*;
        for node in nodes {
            match node {
                DefinitionList { items, .. } => {
                    for item in items {
                        self.do_visit(&item.nodes, func)?;
                    }
                }
                Heading { nodes, .. }
                | Preformatted { nodes, .. }
                | Tag { nodes, .. } => {
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
                            let continue_parsing = func(template)?;
                            if !continue_parsing {
                                return Err(VisitError::StopParsing);
                            }
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

pub fn process_templates_with_function<'lua, R: BufRead>(
    dump_file: R,
    process_template: Function,
    namespaces: HashSet<Namespace>,
    templates: HashSet<String>,
) -> LuaResult<()> {
    let configuration = wiktionary_configuration();
    let parser = dump_parser::parse(dump_file).map(|result| {
        result.unwrap_or_else(|e| {
            panic!("Error while parsing dump: {}", e);
        })
    });
    for page in parser {
        if namespaces.contains(&page.namespace) {
            let wikitext = &page.text;
            let parser_output = configuration.parse(&page.text);
            let continue_parsing = TemplateVisitor::new(wikitext, &templates)
                .visit(
                &parser_output.nodes,
                &mut |template| {
                    Ok(process_template
                        .call((&template, page.title.as_str()))?)
                },
            )?;
            if !continue_parsing {
                break;
            }
        }
    }

    Ok(())
}
