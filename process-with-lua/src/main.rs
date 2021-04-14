use bzip2::bufread::BzDecoder;
use getopts::Options;
use rlua::{
    Context, Function, Lua, Result as LuaResult,
    String as LuaString, ToLua, Value, Variadic,
};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::str::FromStr;
use unicase::UniCase;
use dump_parser::Namespace;

#[macro_export]
macro_rules! exit_with_error {
    ($($tt:tt)*) => ({
        eprintln!($($tt)*);
        ::std::process::exit(-1)
    })
}

mod process_templates;
use process_templates::process_templates_with_function;

mod process_templates_with_headers;
use process_templates_with_headers::process_templates_and_headers_with_function;

mod process_comments_with_headers;
use process_comments_with_headers::process_comments_and_headers_with_function;

mod process_headers;
use process_headers::process_headers_with_function;

struct Page(dump_parser::Page);

impl From<dump_parser::Page> for Page {
    fn from(page: dump_parser::Page) -> Self {
        Page(page)
    }
}

impl<'lua, 'a> ToLua<'lua> for Page {
    fn to_lua(self, lua: Context<'lua>) -> LuaResult<Value<'lua>> {
        let page = self.0;
        let table = lua.create_table()?;
        table.set("title", page.title)?;
        table.set("text", page.text)?;
        let namespace_str = page.namespace.as_str();
        table.set("namespace", namespace_str)?;
        if let Some(format) = page.format {
            table.set("format", format)?;
        }
        if let Some(model) = page.model {
            table.set("model", model)?;
        }
        Ok(Value::Table(table))
    }
}

/// Make a Lua function from an expression or the body of a function,
/// in which `parameters` are the names of the arguments.
fn make_function_from_short_script<'lua>(
    context: Context<'lua>,
    script: &str,
    name: &str,
    parameters: &[&str],
) -> LuaResult<Function<'lua>> {
    let parameters = parameters.as_ref().join(", ");
    let full_script = format!("local {} = ... return {}", &parameters, script);
    let chunk = context.load(&full_script).set_name(&name)?;
    match chunk.into_function() {
        Ok(f) => Ok(f),
        Err(_) => {
            let full_script = format!("local {} = ... {}", &parameters, script);
            let chunk = context.load(&full_script).set_name(&name)?;
            chunk.into_function()
        }
    }
}

/// Run a Lua script that returns a function.
fn make_function<'lua, L>(
    context: Context<'lua>,
    script: &str,
    name: &str,
    script_args: Variadic<L>,
) -> LuaResult<Function<'lua>>
where
    L: ToLua<'lua>,
{
    let chunk = context.load(&script).set_name(&name)?;
    chunk.call(script_args)
}

fn process_text_with_function<R: BufRead>(
    dump_file: R,
    process_page: Function,
    namespaces: HashSet<Namespace>,
) -> LuaResult<()> {
    let parser = dump_parser::parse(dump_file).map(|result| {
        result.unwrap_or_else(|e| {
            exit_with_error!("error while parsing dump: {}", e);
        })
    });
    for page in parser {
        if namespaces.contains(&page.namespace) {
            let continue_parsing: bool = process_page.call(Page::from(page))?;
            if !continue_parsing {
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Subcommand {
    Text,
    Templates,
    TemplatesAndHeaders,
    CommentsAndHeaders,
    Headers,
    Help,
}

impl FromStr for Subcommand {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let subcommand = match s {
            "text" => Subcommand::Text,
            "templates" => Subcommand::Templates,
            "templates-and-headers" => Subcommand::TemplatesAndHeaders,
            "comments-and-headers" => Subcommand::CommentsAndHeaders,
            "headers" => Subcommand::Headers,
            "help" | "--help" | "-h" => Subcommand::Help,
            _ => return Err("unrecognized subcommand"),
        };
        Ok(subcommand)
    }
}
const SUBCOMMANDS: &[&str] = &[
    "text",
    "templates",
    "templates-and-headers",
    "comments-and-headers",
    "headers",
];

fn handle_lua_init(context: Context) -> LuaResult<()> {
    for key in &["LUA_INIT", "LUA_INIT_5_3"] {
        if let Ok(value) = std::env::var(key) {
            if value.as_bytes()[0] == b'@' {
                let filename = &value[1..];
                let script = match File::open(filename) {
                    Ok(mut f) => {
                        let mut contents = String::new();
                        if let Err(e) = f.read_to_string(&mut contents) {
                            exit_with_error!(
                                "could not read file {}: {}",
                                filename,
                                e
                            );
                        }
                        contents
                    }
                    Err(e) => {
                        exit_with_error!(
                            "could not open init script {}: {}",
                            filename,
                            e
                        );
                    }
                };
                context.load(&script).set_name(filename)?.exec()?
            } else {
                let name = format!("={}", key);
                context.load(&value).set_name(&name)?.exec()?
            }
            break;
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args().collect();

    let subcommand: Subcommand = if let Some(arg) = args.get(1) {
        arg.parse().unwrap_or_else(|e| {
            exit_with_error!(
                "{}: {}; choose between {}",
                e,
                args[1],
                SUBCOMMANDS.join(", ")
            );
        })
    } else {
        exit_with_error!("provide subcommand: {}", SUBCOMMANDS.join(", "));
    };

    let mut options = Options::new();
    options.optopt("s", "script", "Lua script", "FILE");
    options.optopt("e", "eval", "Lua code", "TEXT");
    options.optopt("i", "dump", "XML page dump file", "FILE");
    options.optmulti(
        "n",
        "namespaces",
        "list of namespaces (names or numbers) to process",
        "NS",
    );
    options.optmulti("t", "templates", "list of templates", "TEMPLATES");
    options.optmulti(
        "T",
        "template-file",
        "file containing newline-separated template names",
        "TEMPLATES",
    );
    options.optflag("h", "help", "print this help menu");

    if subcommand == Subcommand::Help {
        print!(
            "{}",
            options.usage("Usage: process-with-lua SUBCOMMAND OPTIONS")
        );
        return;
    }

    let matches = match options.parse(&args[2..]) {
        Ok(m) => m,
        Err(e) => exit_with_error!("{}", e.to_string()),
    };

    if (matches.opt_present("templates") || matches.opt_present("template-file")) && !(subcommand == Subcommand::Templates
        || subcommand == Subcommand::TemplatesAndHeaders)
    {
        exit_with_error!("--templates or --template-file only allowed with subcommand templates or templates-and-headers");
    }

    let namespace_args = matches.opt_strs("namespaces");
    let (mut namespaces, mut failures) = (Vec::new(), Vec::<&str>::new());
    for namespace_arg in &namespace_args {
        match namespace_arg.parse::<Namespace>() {
            Ok(n) => namespaces.push(n),
            Err(_) => failures.push(namespace_arg),
        }
    }
    if !failures.is_empty() {
        exit_with_error!(
            "invalid namespace{}: {}",
            if failures.len() == 1 { "" } else { "s" },
            failures.join(", ")
        );
    } else if namespaces.is_empty() {
        namespaces.push(Namespace::Main);
    }

    let (script, name, eval) = if matches.opt_present("eval") {
        let script = matches.opt_str("eval").unwrap();
        let name = "(command line)".to_string();
        (script, name, true)
    } else if matches.opt_present("script") {
        let filename = matches.opt_str("script").unwrap();
        let mut script = String::new();
        File::open(&filename)
            .unwrap_or_else(|e| {
                exit_with_error!(
                    "could not open script file '{}': {}",
                    &filename,
                    e
                )
            })
            .read_to_string(&mut script)
            .unwrap();
        (script, filename, false)
    } else {
        exit_with_error!("Either code or a script file is required.");
    };

    let dump_filename = matches
        .opt_str("dump")
        .unwrap_or_else(|| "pages-articles.xml".into());

    let dump = File::open(&dump_filename).unwrap_or_else(|e| {
        exit_with_error!("could not open dump file '{}': {}", &dump_filename, e)
    });

    let namespaces: HashSet<_> = namespaces.into_iter().collect();

    let templates: Option<HashSet<_>> = if subcommand == Subcommand::Templates
        || subcommand == Subcommand::TemplatesAndHeaders
    {
        let templates: HashSet<_> = if matches.opt_present("templates") {
            if matches.opt_present("template-file") {
                eprintln!("both --template and --template-file provided; --template-file ignored.");
            }
            matches.opt_strs("templates").into_iter().collect()
        } else if let Some(file_path) = matches.opt_str("template-file") {
            let file = File::open(&file_path).unwrap_or_else(|e| {
                exit_with_error!("Could not open {}: {}", file_path, e);
            });
            BufReader::new(file).lines().map(|l| {
                l.unwrap_or_else(|e| {
                    exit_with_error!("Error while reading {}: {}", file_path, e)
                })
            }).collect()
        } else {
            exit_with_error!(
                "Either --templates or --template-file is required"
            );
        };
        Some(templates)
    } else {
        None
    };

    let lua = unsafe { Lua::new_with_debug() };

    lua.context(|ctx| {
        handle_lua_init(ctx)?;

        let casefold_cmp =
            ctx.create_function(|_, (a, b): (LuaString, LuaString)| {
                Ok(UniCase::new(a.to_str()?) < UniCase::new(b.to_str()?))
            })?;

        ctx.globals().set("casefold_cmp", casefold_cmp)?;

        let process_page: Function = if eval {
            let parameters: &[&str] = match subcommand {
                Subcommand::Text => &["page"],
                Subcommand::Templates => &["template", "title"],
                Subcommand::TemplatesAndHeaders => {
                    &["templates", "headers", "title"]
                }
                Subcommand::CommentsAndHeaders => {
                    &["comments", "headers", "title"]
                }
                Subcommand::Headers => &["header", "title"],
                _ => &[],
            };
            make_function_from_short_script(ctx, &script, &name, parameters)
        } else {
            let script_args: Variadic<_> = matches
                .free
                .into_iter()
                .map(|a| a.to_lua(ctx).unwrap())
                .collect();
            make_function(ctx, &script, &name, script_args)
        }?;

        let dump = BufReader::new(dump);
        if dump_filename.ends_with(".bz2") {
            let dump = BzDecoder::new(dump);
            let dump = BufReader::new(dump);
            match subcommand {
                Subcommand::Text => {
                    process_text_with_function(dump, process_page, namespaces)
                }
                Subcommand::Templates => process_templates_with_function(
                    dump,
                    process_page,
                    namespaces,
                    templates.unwrap(),
                ),
                Subcommand::TemplatesAndHeaders => {
                    process_templates_and_headers_with_function(
                        dump,
                        process_page,
                        namespaces,
                        templates.unwrap(),
                    )
                }
                Subcommand::CommentsAndHeaders => {
                    process_comments_and_headers_with_function(
                        dump,
                        process_page,
                        namespaces,
                    )
                }
                Subcommand::Headers => process_headers_with_function(
                    dump,
                    process_page,
                    namespaces,
                ),
                _ => Ok(()),
            }
        } else {
            match subcommand {
                Subcommand::Text => {
                    process_text_with_function(dump, process_page, namespaces)
                }
                Subcommand::Templates => process_templates_with_function(
                    dump,
                    process_page,
                    namespaces,
                    templates.unwrap(),
                ),
                Subcommand::TemplatesAndHeaders => {
                    process_templates_and_headers_with_function(
                        dump,
                        process_page,
                        namespaces,
                        templates.unwrap(),
                    )
                }
                Subcommand::CommentsAndHeaders => {
                    process_comments_and_headers_with_function(
                        dump,
                        process_page,
                        namespaces,
                    )
                }
                Subcommand::Headers => process_headers_with_function(
                    dump,
                    process_page,
                    namespaces,
                ),
                _ => Ok(()),
            }
        }?;

        Ok(())
    })
    .unwrap_or_else(|e: rlua::Error| {
        eprintln!("Error in Lua: {}", e);
    });
}
