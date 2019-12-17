use std::{
    convert::From,
    fs::File,
    io::{BufRead, BufReader, Read},
    str::FromStr,
};
use structopt::StructOpt;
use structopt::clap::{
    AppSettings::ColoredHelp,
    Shell,
};
use bzip2::bufread::BzDecoder;
use wiktionary_namespaces::Namespace;

#[derive(StructOpt)]
#[structopt(name = "wiktionary_data", setting(ColoredHelp), rename_all = "kebab-case")]
pub struct Args {
    #[structopt(long, short)]
    verbose: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    #[structopt(setting(ColoredHelp))]
    DumpTemplates {
        #[structopt(flatten)]
        args: TemplateDumpArgs,
    },
    #[structopt(setting(ColoredHelp))]
    DumpParsedTemplates {
        #[structopt(flatten)]
        args: TemplateDumpArgs,
        #[structopt(long, short)]
        /// format: CBOR or JSON (more precisely JSON Lines)
        format: SerializationFormat,
    },
    #[structopt(setting(ColoredHelp))]
    AllHeaders {
        #[structopt(long, short = "P")]
        /// print pretty JSON
        pretty: bool,
        #[structopt(flatten)]
        dump_args: DumpArgs,
    },
    #[structopt(setting(ColoredHelp))]
    FilterHeaders {
        #[structopt(long = "top-level-headers", short)]
        top_level_header_filepaths: Vec<String>,
        #[structopt(long = "other-headers", short)]
        other_header_filepaths: Vec<String>,
        #[structopt(long, short = "P")]
        /// print pretty JSON
        pretty: bool,
        #[structopt(flatten)]
        dump_args: DumpArgs,
    },
    #[structopt(setting(ColoredHelp))]
    AddTemplateRedirects {
        #[structopt(long, short)]
        suffix: String,
        files: Vec<String>,
    },
    Completions {
        shell: Shell,
    },
}

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

#[derive(StructOpt, Clone)]
struct DumpArgs {
    #[structopt(long, short, value_delimiter = ",", default_value = "main")]
    /// namespace to process
    namespaces: Vec<Namespace>,
    #[structopt(short, long)]
    /// number of pages to process [default: unlimited]
    pages: Option<usize>,
    /// path to pages-articles.xml[.bz2] or pages-meta-current.xml[.bz2]
    #[structopt(long = "input", short = "i")]
    dump_filepath: Option<String>,
}

#[derive(StructOpt)]
struct TemplateDumpArgs {
    #[structopt(long = "templates", short, required = true)]
    /// path to file containing template names with optional tab and output filepath
    template_filepaths: Vec<String>,
    #[structopt(flatten)]
    dump_args: DumpArgs,
}

pub struct Opts {
    pub verbose: bool,
    pub cmd: CommandData,
}

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
        files: Vec<String>,
        suffix: String,
    },
    Completions {
        shell: Shell,
    },
}

pub struct DumpOptions {
    pub pages: usize,
    pub namespaces: Vec<Namespace>,
    pub dump_file: Box<dyn Read>,
}

pub struct TemplateDumpOptions {
    pub files: Vec<(String, Option<String>)>,
    pub dump_options: DumpOptions,
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

enum DumpFileError {
    IoError(std::io::Error),
    DefaultsNotFound,
}

impl From<std::io::Error> for DumpFileError {
    fn from(e: std::io::Error) -> DumpFileError { DumpFileError::IoError(e) }
}

const DEFAULT_DUMP_FILE_NAMES: &[&str] = &[
    "pages-articles.xml",
    "pages-meta-current.xml",
    "pages-articles.xml.bz2",
    "pages-meta-current.xml.bz2",
];

fn get_dump_file(path: &Option<String>) -> Result<Box<dyn Read>, DumpFileError> {
    let (file, path) = if let Some(path) = path {
        (File::open(&path)?, path.as_str())
    } else {
        if let Some((file, path)) = DEFAULT_DUMP_FILE_NAMES.iter()
            .filter_map(|path| {
                if let Ok(f) = File::open(path) {
                    Some((f, path))
                } else {
                    None
                }
            }).next() {
            (file, *path)
        } else {
            return Err(DumpFileError::DefaultsNotFound);
        }
    };
    Ok(if path.ends_with(".bz2") {
        Box::new(BzDecoder::new(BufReader::new(file)))
    } else {
        Box::new(file)
    })
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
            let dump_file = get_dump_file(&dump_filepath).unwrap_or_else(|e| {
                match e {
                    DumpFileError::IoError(e) => {
                        panic!("error while opening dump file: {}", e);
                    }
                    DumpFileError::DefaultsNotFound => {
                        panic!(
                            concat!(
                                "no dump filepath given, and did not find any of the ",
                                "following filenames in the current directory: {}"
                            ),
                            DEFAULT_DUMP_FILE_NAMES.join(", ")
                        )
                    }
                }
            });
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
        Command::Completions { shell } => CommandData::Completions { shell },
    };
    Opts { verbose, cmd }
}