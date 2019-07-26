use parse_wiki_text::{self, Node::{self, Template}, Parameter};

use crate::nodes_ext;
use nodes_ext::get_nodes_text;

/*
fn print_template_parameters (
    page: &Page,
    parameters: &Vec<parse_wiki_text::Parameter>
) {
    let mut parameter_index = 0;
    for parameter in parameters {
        let value = get_nodes_text(page, &parameter.value);
        if let Some(nodes) = &parameter.name {
            println!("|{}={}", get_nodes_text(page, &nodes), value);
        } else {
            parameter_index += 1;
            println!("|{}={}", parameter_index, value)
        }
    }
}
*/

fn get_integer_parameter<'a> (wikitext: &'a str, parameters: &'a Vec<Parameter<'a>>, key: &str)
    -> Option<&'a Parameter<'a>>
{
    let key_number = key.parse::<u32>().unwrap();
    let mut parameter_index = 0;
    let mut value = None;
    for parameter in parameters {
        match parameter.name {
            Some(ref name) if get_nodes_text(wikitext, name) == key => {
                value = Some(parameter);
            },
            None => {
                parameter_index += 1;
                if parameter_index == key_number {
                    value = Some(parameter);
                }
            },
            _ => (),
        }
    }
    value
}

fn get_other_parameter<'a> (wikitext: &'a str, parameters: &'a Vec<Parameter<'a>>, key: &str)
    -> Option<&'a Parameter<'a>>
{
    let mut value = None;
    for parameter in parameters {
        match parameter.name {
            Some(ref name) if get_nodes_text(wikitext, name) == key => {
                value = Some(parameter);
            },
            _ => (),
        }
    }
    value
}

pub fn get_parameter<'a> (wikitext: &'a str, template: &'a Node<'a>, key: &str)
    -> Result<Option<&'a Parameter<'a>>, &'static str>
{
    if let Template { parameters, .. } = template {
        Ok(
            if key.bytes().all(|b| b.is_ascii_digit()) {
                get_integer_parameter(wikitext, &parameters, key)
            } else {
                get_other_parameter(wikitext, &parameters, key)
            }
        )
    } else {
        Err("node is not a template")
    }
}

#[derive(Debug)]
pub enum ParameterKey<'a> {
    NodeList(&'a Vec<Node<'a>>),
    Number(u32),
}

pub fn enumerate<'a>(template: &'a Node<'a>)
    -> Result<impl Iterator<Item=(ParameterKey<'a>, &'a Vec<Node<'a>>)>, &'static str>
{
    if let Template { parameters, .. } = template {
        let mut parameter_index = 0;
        Ok(
            parameters.iter()
                .map(move |p| {
                    let value = &p.value;
                    if let Parameter { name: Some(name), .. } = p {
                        (ParameterKey::NodeList(name), value)
                    } else {
                        parameter_index += 1;
                        (ParameterKey::Number(parameter_index), value)
                    }
                })
        )
    } else {
        Err("node is not a template")
    }
}

pub fn get_parameter_value<'a> (
    wikitext: &'a str,
    template: &'a Node<'a>,
    key: &str
) -> Result<Option<&'a Vec<Node<'a>>>, &'static str> {
    let key_number = if key.bytes().all(|b| b.is_ascii_digit()) {
        Some(key.parse::<u32>().unwrap())
    } else {
        None
    };
    enumerate(template)
        .map(|iter| {
            iter.filter(|(k, _)| {
                match k {
                    ParameterKey::NodeList(name) => {
                        get_nodes_text(wikitext, name) == key
                    },
                    ParameterKey::Number(index) => {
                        key_number.is_some() && key_number.unwrap() == *index
                    },
                }
            })
            .last()
            .map(|(_, nodes)| nodes)
        })
}

#[cfg(test)]
mod tests {
    use parse_wiki_text;
    use crate::configuration;
    use super::{get_parameter, enumerate, ParameterKey};
    use crate::nodes_ext::get_nodes_text;
    use parse_wiki_text::{Node, Parameter};
    
    #[derive(Debug, Eq, PartialEq)]
    enum Key<'a> {
        Integer(u32),
        String(&'a str)
    }
    
    fn show_parameter_key_and_value<'a>(
        wikitext: &'a str,
        (key, value): (ParameterKey<'a>, &'a Vec<Node<'a>>)
    ) -> (Key<'a>, &'a str)
    {
        (
            match key {
                ParameterKey::NodeList(list) => Key::String(get_nodes_text(wikitext, list)),
                ParameterKey::Number(num) => Key::Integer(num),
            },
            get_nodes_text(wikitext, &value)
        )
    }
    
    #[test]
    fn test_enumerate() {
        let template_text = "{{test|one a|two|1=one b|{{test}}=template}}";
        let output = configuration::create().parse(template_text);
        assert_eq!(output.warnings.len(), 0);
        assert_eq!(output.nodes.len(), 1);
        let template = &output.nodes[0];
        let mut keys_and_values = enumerate(template).unwrap();
        assert_eq!(
            show_parameter_key_and_value(template_text, keys_and_values.next().unwrap()),
            (Key::Integer(1), "one a")
        );
        assert_eq!(
            show_parameter_key_and_value(template_text, keys_and_values.next().unwrap()),
            (Key::Integer(2), "two")
        );
        assert_eq!(
            show_parameter_key_and_value(template_text, keys_and_values.next().unwrap()),
            (Key::String("1"), "one b")
        );
        assert_eq!(
            show_parameter_key_and_value(template_text, keys_and_values.next().unwrap()),
            (Key::String("{{test}}"), "template")
        );
    }
    
    fn get_parameter_text<'a>(wikitext: &'a str, template: &'a Node<'a>, key: &str)
        -> Option<&'a str>
    {
        match get_parameter(wikitext, &template, key) {
            Ok(p) => {
                match p {
                    Some(Parameter { value: v, .. }) => Some(get_nodes_text(wikitext, v)),
                    None => None,
                }
            },
            Err(_) => None,
        }
    }
    
    #[test]
    fn test_get_parameter() {
        let template_text = "{{test|3=three a|one a|two|three b|1=one b|{{test}}=template}}";
        let output = configuration::create().parse(template_text);
        assert_eq!(output.warnings.len(), 0);
        assert_eq!(output.nodes.len(), 1);
        let template = &output.nodes[0];
        
        assert_eq!(
            get_parameter_text(template_text, &template, "1"),
            Some("one b")
        );
        
        assert_eq!(
            get_parameter_text(template_text, &template, "2"),
            Some("two")
        );
        
        assert_eq!(
            get_parameter_text(template_text, &template, "3"),
            Some("three b")
        );
        
        assert_eq!(
            get_parameter_text(template_text, &template, "{{test}}"),
            Some("template")
        );
    }
}