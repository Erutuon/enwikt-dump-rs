use std::{
    convert::TryFrom,
    fmt::Write as WriteFmt,
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
    time::{Duration, Instant},
};
use structopt::StructOpt;

mod nodes_ext;

mod namespace;
use namespace::Namespace;

mod dump_parser;
use dump_parser::parse as parse_dump;

mod template_dumper;
use template_dumper::TemplateDumper;

fn parse_namespace (namespace: &str) -> Result<u32, &str> {
    if let Ok(n) = u32::from_str(namespace) {
        Ok(n)
    } else {
        Namespace::from_str(namespace).map(u32::from)
    }
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Args {
    #[structopt(long = "templates", short)]
    /// path to file containing template names with optional tab and output filepath
    template_filepaths: Vec<String>,
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
    #[structopt(long, short)]
    verbose: bool,
}

#[derive(Debug)]
struct Opts {
    pages: usize,
    files: Vec<(String, Option<String>)>,
    namespaces: Vec<Namespace>,
    dump_file: File,
    verbose: bool,
}

fn collect_template_names_and_files(template_filepaths: Vec<String>)
    -> Vec<(String, Option<String>)>
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

fn get_opts() -> Opts {
    let args = Args::from_args();
    let Args { template_filepaths, namespaces, pages, dump_filepath, verbose } = args;
    let mut namespaces: Vec<Namespace> = namespaces.iter()
        .map(|n| Namespace::try_from(*n).unwrap_or_else(|_| {
            panic!("{} is not a valid namespace id", n)
        }))
        .collect();
    if dbg!(&namespaces).is_empty() {
        namespaces.push(Namespace::Main);
    }
    let pages = pages.unwrap_or(std::usize::MAX);
    let files = collect_template_names_and_files(template_filepaths);
    let dump_file = File::open(dump_filepath).unwrap_or_else(|e|
        panic!("did not find pages-articles.xml: {}", e)
    );
    Opts { pages, namespaces, files, dump_file, verbose }
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
                0...2 => &decimals[..3],
                3...5 => &decimals[..6],
                _     => &decimals[..9],
            }
        } else {
            &decimals[..3]
        }
    });
    write!(printed, "s").unwrap();
    printed
}

fn main() {
    let main_start = Instant::now();
    let opts = get_opts();
    let parser = parse_dump(opts.dump_file);
    let mut dumper = TemplateDumper::new(opts.files);
    let start_time = main_start.elapsed();
    let parse_start = Instant::now();
    dumper.parse(parser, opts.pages, opts.namespaces, opts.verbose);
    let parse_time = parse_start.elapsed();
    eprintln!("startup took {}, parsing {}",
        print_time(&start_time),
        print_time(&parse_time));
}