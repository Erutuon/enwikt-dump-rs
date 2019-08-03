use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::Write as WriteFmt,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    str::FromStr,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use serde::Serialize;
use serde_json::error::Error as SerdeJsonError;
use unicase::UniCase;

mod namespace;
use namespace::Namespace;

mod dump_parser;
use dump_parser::parse as parse_dump;

mod template_dumper;
use template_dumper::TemplateDumper;

mod header_stats;
use header_stats::HeaderStats;

mod filter_headers;
use filter_headers::HeaderFilterer;

fn parse_namespace (namespace: &str) -> Result<u32, &str> {
    if let Ok(n) = u32::from_str(namespace) {
        Ok(n)
    } else {
        Namespace::from_str(namespace).map(u32::from)
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "wiktionary_data", raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Args {
    #[structopt(long, short)]
    verbose: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(
        raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
        name = "dump_templates",
    )]
    DumpTemplates {
        #[structopt(long = "templates", short)]
        /// path to file containing template names with optional tab and output filepath
        template_filepaths: Vec<String>,
        #[structopt(flatten)]
        dump_args: DumpArgs,
    },
    #[structopt(
        name = "all_headers",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
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
        raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
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
        raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
    )]
    AddTemplateRedirects {
        files: Vec<String>,
    },
}

#[derive(StructOpt, Debug, Clone)]
struct DumpArgs {
    #[structopt(
        long = "namespace",
        short,
        parse(try_from_str = "parse_namespace"),
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

#[derive(Debug)]
struct Opts {
    verbose: bool,
    cmd: CommandData,
}

#[derive(Debug)]
enum CommandData {
    DumpTemplates {
        files: Vec<(String, Option<String>)>,
        dump_options: DumpOptions,
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
    },
}

#[derive(Debug)]
struct DumpOptions {
    pages: usize,
    namespaces: Vec<Namespace>,
    dump_file: File,
}

fn collect_template_names_and_files<I, S> (template_filepaths: I)
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

fn get_opts() -> Opts {
    let args = Args::from_args();
    let Args { verbose, cmd } = args;
    let dump_options = match &cmd {
          Command::DumpTemplates { dump_args, .. }
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
    
    let cmd = match cmd {
        Command::DumpTemplates { template_filepaths, .. } => CommandData::DumpTemplates {
            files: collect_template_names_and_files(&template_filepaths),
            dump_options: dump_options.unwrap(),
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
        Command::AddTemplateRedirects { files } => CommandData::AddTemplateRedirects { files },
    };
    Opts { verbose, cmd }
}

fn print_time(time: &Duration) -> String {
    let nanos = time.subsec_nanos();
    let mut secs = time.as_secs();
    let mins = secs / 60;
    let mut printed = String::new();
    if mins > 0 {
        secs = secs % 60;
        write!(printed, "{}m ", mins).unwrap();
    }
    write!(printed, "{}.", secs).unwrap();
    let decimals = format!("{:09}", nanos);
    printed.push_str({
        if secs == 0 && mins == 0 {
            let zero_count = decimals.as_bytes().iter()
                .take_while(|&&b| b == b'0')
                .count();
            match zero_count {
                0..=2 => &decimals[..3],
                3..=5 => &decimals[..6],
                _     => &decimals[..9],
            }
        } else {
            &decimals[..3]
        }
    });
    write!(printed, "s").unwrap();
    printed
}

fn do_dumping<S>(dumper: &S, pretty: bool) -> Result<(), SerdeJsonError>
    where S: Serialize
{
    if pretty {
        serde_json::to_writer_pretty(std::io::stdout().lock(), &dumper)
    } else {
        match serde_json::to_string(&dumper) {
            Ok(printed) => {
                let printed = printed.replace("{", "\n\t{").replace("}]", "}\n]");
                println!("{}", &printed);
                Ok(())
            },
            Err(e) => {
                Err(e)
            },
        }
    }
}

fn main() {
    let main_start = Instant::now();
    let opts = get_opts();
    let verbose = opts.verbose;
    match opts.cmd {
        CommandData::DumpTemplates { files, dump_options: opts } => {
            let parser = parse_dump(opts.dump_file);
            let mut dumper = TemplateDumper::new(files);
            dumper.add_redirects();
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            dumper.parse(parser, opts.pages, opts.namespaces, verbose);
            let parse_time = parse_start.elapsed();
            eprintln!("startup took {}, parsing {}",
                print_time(&start_time),
                print_time(&parse_time));
        },
        CommandData::AllHeaders { pretty, dump_options: opts } => {
            let parser = parse_dump(opts.dump_file);
            let mut dumper = HeaderStats::new();
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            dumper.parse(parser, opts.pages, opts.namespaces, verbose);
            do_dumping(&dumper, pretty).unwrap_or_else(|e| eprintln!("{}", e));
            let parse_time = parse_start.elapsed();
            eprintln!("startup took {}, parsing and printing {}",
                print_time(&start_time),
                print_time(&parse_time));
        },
        CommandData::FilterHeaders { top_level_headers, other_headers, pretty, dump_options: opts } => {
            let parser = parse_dump(opts.dump_file);
            let mut filterer = HeaderFilterer::new(top_level_headers, other_headers);
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            filterer.parse(parser, opts.pages, opts.namespaces, verbose);
            do_dumping(&filterer, pretty).unwrap_or_else(|e| eprintln!("{}", e));
            let parse_time = parse_start.elapsed();
            eprintln!("startup took {}, parsing and printing {}",
                print_time(&start_time),
                print_time(&parse_time));
        },
        CommandData::AddTemplateRedirects { files } => {
            for path in files {
                let mut template_names_and_files: HashMap<_, _> =
                    collect_template_names_and_files(&[&path])
                    .into_iter()
                    .map(|(template, filepath)| {
                        let filepath = filepath.unwrap_or_else(|| {
                            format!("{}.txt", template)
                        });
                        (template, filepath)
                    })
                    .collect();
                template_dumper::add_template_redirects(&mut template_names_and_files);
                let mut template_names_and_files: Vec<_> = template_names_and_files
                    .into_iter()
                    .collect();
                template_names_and_files.sort_by(|(a, _), (b, _)| UniCase::new(a).cmp(&UniCase::new(b)));
                let mut file = BufWriter::new(File::create(&format!("{}.new", path)).unwrap());
                for (a, b) in template_names_and_files {
                    write!(file, "{}\t{}\n", a, b).unwrap();
                }
            }
        },
    }
}