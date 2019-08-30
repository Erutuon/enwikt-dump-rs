use std::{
    convert::TryFrom,
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
};
use structopt::{StructOpt, clap::AppSettings::ColoredHelp};
use wiktionary_namespaces::Namespace;

#[derive(StructOpt, Debug)]
#[structopt(name = "wiktionary_data", setting(ColoredHelp))]
struct Args {
    #[structopt(long, short)]
    verbose: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(
        setting(ColoredHelp),
        name = "dump_templates",
    )]
    DumpTemplates {
        #[structopt(flatten)]
        args: TemplateDumpArgs,
    },
    #[structopt(
        name = "dump_parsed_templates",
        setting(ColoredHelp),
    )]
    DumpParsedTemplates {
        #[structopt(flatten)]
        args: TemplateDumpArgs,
    },
    #[structopt(
        name = "all_headers",
        setting(ColoredHelp),
    )]
    AllHeaders {
        #[structopt(long, short = "P")]
        /// print pretty JSON
        pretty: bool,
        #[structopt(flatten)]
        dump_args: DumpArgs,
    },
    #[structopt(
        name = "filter_headers",
        setting(ColoredHelp),
    )]
    FilterHeaders {
        #[structopt(long = "top_level_header", short)]
        top_level_header_filepaths: Vec<String>,
        #[structopt(long = "other_headers", short)]
        other_header_filepaths: Vec<String>,
        #[structopt(long, short = "P")]
        /// print pretty JSON
        pretty: bool,
        #[structopt(flatten)]
        dump_args: DumpArgs,
    },
    #[structopt(
        name = "add_template_redirects",
        setting(ColoredHelp),
    )]
    AddTemplateRedirects {
        #[structopt(long, short)]
        suffix: String,
        files: Vec<String>,
    },
}

#[derive(StructOpt, Debug, Clone)]
struct DumpArgs {
    #[structopt(
        long = "namespace",
        short,
        parse(try_from_str = parse_namespace),
        value_delimiter = ",",
        default_value = "main",
    )]
    /// namespace to process
    namespaces: Vec<u32>,
    #[structopt(short, long)]
    /// number of pages to process [default: unlimited]
    pages: Option<usize>,
    /// path to pages-articles.xml or pages-meta-current.xml
    #[structopt(long = "input", short = "i", default_value = "pages-articles.xml")]
    dump_filepath: String,
}

#[derive(Debug, StructOpt)]
struct TemplateDumpArgs {
    #[structopt(long = "templates", short)]
    /// path to file containing template names with optional tab and output filepath
    template_filepaths: Vec<String>,
    #[structopt(flatten)]
    dump_args: DumpArgs,
}

#[derive(Debug)]
pub struct Opts {
    pub verbose: bool,
    pub cmd: CommandData,
}

#[derive(Debug)]
pub enum CommandData {
    DumpTemplates {
        options: TemplateDumpOptions,
    },
    DumpParsedTemplates {
        options: TemplateDumpOptions,
    },
    AllHeaders {
        pretty: bool,
        dump_options: DumpOptions,
    },
    FilterHeaders {
        top_level_headers: Vec<String>,
        other_headers: Vec<String>,
        pretty: bool,
        dump_options: DumpOptions,
    },
    AddTemplateRedirects {
        files: Vec<String>,
        suffix: String,
    },
}

#[derive(Debug)]
pub struct DumpOptions {
    pub pages: usize,
    pub namespaces: Vec<Namespace>,
    pub dump_file: File,
}

#[derive(Debug)]
pub struct TemplateDumpOptions {
    pub files: Vec<(String, Option<String>)>,
    pub dump_options: DumpOptions,
}

fn parse_namespace (namespace: &str) -> Result<u32, &str> {
    if let Ok(n) = u32::from_str(namespace) {
        Ok(n)
    } else {
        Namespace::from_str(namespace).map(u32::from)
    }
}

pub fn collect_template_names_and_files<I, S> (template_filepaths: I)
    -> Vec<(String, Option<String>)>
    where
        I: IntoIterator<Item = S>,
        S: std::convert::AsRef<std::path::Path> + std::fmt::Display,
{
    let mut template_and_file: Vec<(String, Option<String>)> = Vec::new();
    for template_filepath in template_filepaths {
        let file = File::open(&template_filepath).unwrap_or_else(|e| {
            panic!("could not open file {}: {}", template_filepath, e);
        });
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line.unwrap_or_else(|e| {
                panic!("error while reading line {} in {}: {}",
                    i, template_filepath, e);
            });
            let mut iter = line.splitn(2, '\t');
            let template = iter.next().unwrap_or_else(|| {
                panic!("could not split line {} in {}",
                    i, template_filepath);
            }).to_string();
            let filepath = iter.next().map(ToString::to_string);
            template_and_file.push((template, filepath));
        }
    }
    template_and_file
}

fn collect_lines (filepaths: Vec<String>) -> Vec<String> {
    filepaths
        .into_iter()
        .flat_map(
            |path| {
                let file = File::open(&path).unwrap_or_else(|e| {
                    panic!("could not open file {}: {}", &path, e);
                });
                BufReader::new(file).lines().map(|line| {
                    line.unwrap_or_else(|e| {
                        panic!("failed to unwrap line: {}", e);
                    }).to_string()
                })
            }
        )
        .collect()
}

pub fn get_opts() -> Opts {
    let args = Args::from_args();
    let Args { verbose, cmd } = args;
    let dump_options = match &cmd {
          Command::DumpTemplates { args: TemplateDumpArgs { dump_args, .. }, .. }
        | Command::DumpParsedTemplates { args: TemplateDumpArgs { dump_args, .. }, .. }
        | Command::AllHeaders    { dump_args, .. }
        | Command::FilterHeaders { dump_args, .. } => {
            let DumpArgs { namespaces, pages, dump_filepath } = dump_args;
            let mut namespaces: Vec<Namespace> = namespaces.iter()
                .map(|n| Namespace::try_from(*n).unwrap_or_else(|_| {
                    panic!("{} is not a valid namespace id", n)
                }))
                .collect();
            if namespaces.is_empty() {
                namespaces.push(Namespace::Main);
            }
            let pages = pages.unwrap_or(std::usize::MAX);
            let dump_file = File::open(dump_filepath).unwrap_or_else(|e|
                panic!("did not find pages-articles.xml: {}", e)
            );
            Some(DumpOptions { namespaces, pages, dump_file })
        },
        _ => None,
    };
    
    let template_names_and_files = match &cmd {
          Command::DumpTemplates { args }
        | Command::DumpParsedTemplates { args } => {
            Some(collect_template_names_and_files(&args.template_filepaths))
        },
        _ => None,
    };
    
    let cmd = match cmd {
        Command::DumpTemplates { .. } => CommandData::DumpTemplates {
            options: TemplateDumpOptions {
                files: template_names_and_files.unwrap(),
                dump_options: dump_options.unwrap(),
            },
        },
        Command::DumpParsedTemplates { .. } => CommandData::DumpParsedTemplates {
            options: TemplateDumpOptions {
                files: template_names_and_files.unwrap(),
                dump_options: dump_options.unwrap(),
            },
        },
        Command::AllHeaders { pretty, .. } => CommandData::AllHeaders {
            pretty,
            dump_options: dump_options.unwrap(),
        },
        Command::FilterHeaders {
            top_level_header_filepaths,
            other_header_filepaths,
            pretty,
            ..
        } => CommandData::FilterHeaders {
            top_level_headers: collect_lines(top_level_header_filepaths),
            other_headers: collect_lines(other_header_filepaths),
            pretty,
            dump_options: dump_options.unwrap(),
        },
        Command::AddTemplateRedirects { files, suffix } =>
            CommandData::AddTemplateRedirects { files, suffix },
    };
    Opts { verbose, cmd }
}