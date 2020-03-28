use bzip2::bufread::BzDecoder;
use std::{
    collections::HashMap,
    convert::From,
    fs::File,
    io::{BufRead, BufReader, Read},
    rc::Rc,
    str::FromStr,
};
use structopt::clap::{AppSettings::ColoredHelp, Shell};
use structopt::StructOpt;
use wiktionary_namespaces::Namespace;

#[derive(StructOpt)]
#[structopt(
    name = "wiktionary_data",
    setting(ColoredHelp),
    rename_all = "kebab-case"
)]
pub struct Args {
    #[structopt(long, short)]
    verbose: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    #[structopt(setting(ColoredHelp))]
    DumpParsedTemplates {
        #[structopt(long, short)]
        /// format: cbor (CBOR stream) or json (JSON Lines)
        format: SerializationFormat,
        #[structopt(long = "templates", short, required = true)]
        /// path to file containing template names with optional tab and output filepath
        template_filepaths: Vec<String>,
        #[structopt(long, short = "I")]
        /// whether to include source code of templates
        include_text: bool,
        #[structopt(long = "template-normalizations", short = "T")]
        /// JSON file mapping from template name to an array of aliases.
        template_normalization_filepath: Option<String>,
        #[structopt(flatten)]
        dump_args: DumpArgs,
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
    Completions { shell: Shell },
}

pub enum SerializationFormat {
    Cbor,
    Json,
}

impl FromStr for SerializationFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let format = match s.to_lowercase().as_str() {
            "json" => SerializationFormat::Json,
            "cbor" => SerializationFormat::Cbor,
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

pub struct Opts {
    pub verbose: bool,
    pub cmd: CommandData,
}

pub enum CommandData {
    DumpParsedTemplates(DumpParsedTemplates),
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
    Completions {
        shell: Shell,
    },
}

pub struct DumpParsedTemplates {
    pub format: SerializationFormat,
    pub files: Vec<(String, Option<String>)>,
    pub template_normalizations: Option<HashMap<String, Rc<str>>>,
    pub include_text: bool,
    pub dump_options: DumpOptions,
}

pub struct DumpOptions {
    pub pages: usize,
    pub namespaces: Vec<Namespace>,
    pub dump_file: Box<dyn Read>,
}

pub fn collect_template_names_and_files<I, S>(
    template_filepaths: I,
) -> Vec<(String, Option<String>)>
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
                panic!(
                    "error while reading line {} in {}: {}",
                    i, template_filepath, e
                );
            });
            let mut iter = line.splitn(2, '\t');
            let template = iter
                .next()
                .unwrap_or_else(|| {
                    panic!(
                        "could not split line {} in {}",
                        i, template_filepath
                    );
                })
                .to_string();
            let filepath = iter.next().map(ToString::to_string);
            template_and_file.push((template, filepath));
        }
    }
    template_and_file
}

fn collect_lines(filepaths: Vec<String>) -> Vec<String> {
    filepaths
        .into_iter()
        .flat_map(|path| {
            let file = File::open(&path).unwrap_or_else(|e| {
                panic!("could not open file {}: {}", &path, e);
            });
            BufReader::new(file).lines().map(|line| {
                line.unwrap_or_else(|e| {
                    panic!("failed to unwrap line: {}", e);
                })
            })
        })
        .collect()
}

enum DumpFileError {
    IoError(std::io::Error),
    DefaultsNotFound,
}

impl From<std::io::Error> for DumpFileError {
    fn from(e: std::io::Error) -> DumpFileError {
        DumpFileError::IoError(e)
    }
}

const DEFAULT_DUMP_FILE_NAMES: &[&str] = &[
    "pages-articles.xml",
    "pages-meta-current.xml",
    "pages-articles.xml.bz2",
    "pages-meta-current.xml.bz2",
];

fn get_dump_file(
    path: &Option<String>,
) -> Result<Box<dyn Read>, DumpFileError> {
    let (file, path) = if let Some(path) = path {
        (File::open(&path)?, path.as_str())
    } else if let Some((file, path)) = DEFAULT_DUMP_FILE_NAMES
        .iter()
        .filter_map(|path| {
            if let Ok(f) = File::open(path) {
                Some((f, path))
            } else {
                None
            }
        })
        .next()
    {
        (file, *path)
    } else {
        return Err(DumpFileError::DefaultsNotFound);
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
        Command::DumpParsedTemplates { dump_args, .. }
        | Command::AllHeaders { dump_args, .. }
        | Command::FilterHeaders { dump_args, .. } => {
            let DumpArgs {
                namespaces,
                pages,
                dump_filepath,
            } = dump_args;
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
            Some(DumpOptions {
                namespaces: namespaces.to_vec(),
                pages,
                dump_file,
            })
        }
        _ => None,
    };

    let template_names_and_files = match &cmd {
        Command::DumpParsedTemplates {
            template_filepaths, ..
        } => Some(collect_template_names_and_files(template_filepaths)),
        _ => None,
    };

    let template_normalizations = match &cmd {
        Command::DumpParsedTemplates {
            template_normalization_filepath:
                Some(template_normalization_filepath),
            ..
        } => {
            let file = File::open(&template_normalization_filepath)
                .expect("error while opening template normalization file");
            let normalizations: HashMap<String, Vec<String>> =
                serde_json::from_reader(&file).unwrap_or_else(|e| {
                    panic!(
                        "template normalization file {} could not be parsed: {}",
                        template_normalization_filepath,
                        e
                    );
                });
            let capacity = normalizations.iter().map(|(_k, v)| v.len()).sum();
            let normalizations = normalizations.into_iter().fold(
                HashMap::with_capacity(capacity),
                |mut map, (template, aliases)| {
                    let template = template.into();
                    map.extend(
                        aliases
                            .into_iter()
                            .map(|alias| (alias, Rc::clone(&template))),
                    );
                    map
                },
            );
            Some(normalizations)
        }
        _ => None,
    };

    let cmd = match cmd {
        Command::DumpParsedTemplates {
            format,
            include_text,
            ..
        } => {
            let files = template_names_and_files.unwrap();
            let dump_options = dump_options.unwrap();
            CommandData::DumpParsedTemplates(DumpParsedTemplates {
                files,
                dump_options,
                template_normalizations,
                include_text,
                format,
            })
        }
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
        Command::Completions { shell } => CommandData::Completions { shell },
    };
    Opts { verbose, cmd }
}
