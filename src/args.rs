use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
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
        #[structopt(long, short)]
        /// format: CBOR or JSON (more precisely JSON Lines)
        format: SerializationFormat,
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
        #[structopt(long = "top_level_header", short, parse(from_os_str))]
        top_level_header_filepaths: Vec<PathBuf>,
        #[structopt(long = "other_headers", short, parse(from_os_str))]
        other_header_filepaths: Vec<PathBuf>,
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
        #[structopt(parse(from_os_str))]
        files: Vec<PathBuf>,
    },
}

#[derive(Debug)]
pub enum SerializationFormat {
    CBOR,
    JSON,
}

impl FromStr for SerializationFormat {
    type Err = &'static str;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let format = match s.to_lowercase().as_str() {
            "json" => SerializationFormat::JSON,
            "cbor" => SerializationFormat::CBOR,
            _ => return Err("unrecognized format"),
        };
        Ok(format)
    }
}

#[derive(StructOpt, Debug, Clone)]
struct DumpArgs {
    #[structopt(long, short, value_delimiter = ",", default_value = "main")]
    /// namespace to process
    namespaces: Vec<Namespace>,
    #[structopt(short, long)]
    /// number of pages to process [default: unlimited]
    pages: Option<usize>,
    /// path to pages-articles.xml or pages-meta-current.xml
    #[structopt(long = "input", short = "i", default_value = "pages-articles.xml", parse(from_os_str))]
    dump_filepath: PathBuf,
}

#[derive(Debug, StructOpt)]
struct TemplateDumpArgs {
    #[structopt(long = "templates", short, required = true, parse(from_os_str))]
    /// path to file containing template names with optional tab and output filepath
    template_filepaths: Vec<PathBuf>,
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
        format: SerializationFormat,
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
        files: Vec<PathBuf>,
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

pub fn collect_template_names_and_files<I, S> (template_filepaths: I)
    -> Vec<(String, Option<String>)>
    where
        I: IntoIterator<Item = S>,
        S: std::convert::AsRef<Path>,
{
    let mut template_and_file: Vec<(String, Option<String>)> = Vec::new();
    for template_filepath in template_filepaths {
        let file = File::open(&template_filepath).unwrap_or_else(|e| {
            panic!("could not open file {}: {}", template_filepath.as_ref().to_string_lossy(), e);
        });
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line.unwrap_or_else(|e| {
                panic!("error while reading line {} in {}: {}",
                    i, template_filepath.as_ref().to_string_lossy(), e);
            });
            let mut iter = line.splitn(2, '\t');
            let template = iter.next().unwrap_or_else(|| {
                panic!("could not split line {} in {}",
                    i, template_filepath.as_ref().to_string_lossy());
            }).to_string();
            let filepath = iter.next().map(ToString::to_string);
            template_and_file.push((template, filepath));
        }
    }
    template_and_file
}

fn collect_lines (filepaths: Vec<PathBuf>) -> Vec<String> {
    filepaths
        .into_iter()
        .flat_map(
            |path| {
                let file = File::open(&path).unwrap_or_else(|e| {
                    panic!("could not open file {}: {}", path.to_string_lossy(), e);
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
            let pages = pages.unwrap_or(std::usize::MAX);
            let dump_file = File::open(dump_filepath).unwrap_or_else(|e|
                panic!("did not find pages-articles.xml: {}", e)
            );
            Some(DumpOptions { namespaces: namespaces.to_vec(), pages, dump_file })
        },
        _ => None,
    };
    
    let template_names_and_files = match &cmd {
          Command::DumpTemplates { args }
        | Command::DumpParsedTemplates { args, .. } => {
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
        Command::DumpParsedTemplates { format, .. } => CommandData::DumpParsedTemplates {
            options: TemplateDumpOptions {
                files: template_names_and_files.unwrap(),
                dump_options: dump_options.unwrap(),
            },
            format,
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