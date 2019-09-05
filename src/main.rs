use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    ffi::OsStr,
    fmt::Write as WriteFmt,
    fs::File,
    io::{BufWriter, Write},
    rc::Rc,
    time::{Duration, Instant},
};
use serde::{Serialize, Deserialize};
use serde_json::{self, error::Error as SerdeJsonError};
use serde_cbor;
use unicase::UniCase;
use dump_parser::{Page, parse as parse_dump, Warning, parse_wiki_text::Positioned};
use template_dumper::{TemplateDumper, MAX_TEMPLATE_NAME, normalize_template_name};
use header_stats::HeaderStats;
use filter_headers::HeaderFilterer;
use template_iter::{TemplateBorrowed, TemplateVisitor};

mod args;
use args::{CommandData, TemplateDumpOptions, DumpOptions, SerializationFormat};

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
        serde_json::to_writer(std::io::stdout().lock(), &dumper)
    }
}

fn print_parser_warnings(page: &Page, warnings: &Vec<Warning>) {
    for warning in warnings {
        let Warning { start, end, message } = warning;
        let range = 0..page.text.len();
        let message = message.message().trim_end_matches(".");
        if !(range.contains(&start) && range.contains(&end)) {
            eprintln!("byte position {} or {} in warning {} is out of range of {:?}, size of [[{}]]",
                start, end, message, range, &page.title);
        } else {
            eprintln!("{} at bytes {}..{} ({:?}) in [[{}]]",
                &message,
                start, end, &page.text[*start..*end], &page.title);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplateWithText {
    name: String,
    parameters: BTreeMap<String, String>,
    text: String,
}

impl TemplateWithText {
    fn new<S>(wikitext: S, template: TemplateBorrowed) -> Self
        where S: Into<String>
    {
        let parameters = template.parameters
            .iter()
            .map(|(k, v)| (k.to_owned().into(), v.to_owned().into()))
            .collect();
        TemplateWithText {
            name: template.name.into(),
            parameters,
            text: wikitext.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplatesInPage {
    title: String,
    templates: Vec<TemplateWithText>,
}

fn dump_parsed_templates(
    opts: TemplateDumpOptions,
    main_start: Instant,
    verbose: bool,
    format: SerializationFormat,
) {
    let TemplateDumpOptions { files: template_to_file, dump_options: opts } = opts;
    let DumpOptions { pages, namespaces, dump_file } = opts;
    let parser = parse_dump(dump_file)
        .map(|result| {
            result.unwrap_or_else(|e| {
                panic!("Error while parsing dump: {}", e);
            })
        })
        .filter(|ref page| namespaces.contains(&page.namespace.try_into().unwrap()))
        .take(pages);
    type ShareableFileWriter = Rc<RefCell<BufWriter<File>>>;
    let mut files: HashMap<String, (ShareableFileWriter, usize)> = HashMap::new();
    let extension = ".cbor";
    let mut file_count = 0;
    let template_to_file: HashMap<_, _> = template_to_file
        .into_iter()
        .filter_map(|(template, filepath)| {
            if template.len() > MAX_TEMPLATE_NAME {
                None
            } else {
                let mut template_name = [0u8; MAX_TEMPLATE_NAME];
                let normalized = match normalize_template_name(&template, &mut template_name) {
                    Some(n) => n,
                    None => return None,
                };
                let filepath = match filepath {
                    Some(p) => p.to_string(),
                    None => normalized.to_string() + extension,
                };
                Some((
                    normalized.to_string(),
                    match files.get(&filepath) {
                        Some(f) => f.clone(),
                        None => {
                            let file = File::create(&filepath).unwrap_or_else(|e| {
                                panic!("error while creating file {}: {}", &filepath, e);
                            });
                            let file_ref = Rc::new(RefCell::new(BufWriter::new(file)));
                            let file_and_number = (file_ref, file_count);
                            file_count += 1;
                            let cloned = file_and_number.clone();
                            files.insert(filepath, cloned);
                            file_and_number
                        }
                    }
                ))
            }
        })
        .collect();
    let configuration = dump_parser::wiktionary_configuration();
    let file_number_to_file: HashMap<_, _> = files
        .into_iter()
        .map(|(_path, (file, number))| (number, file))
        .collect();
    let start_time = main_start.elapsed();
    let parse_start = Instant::now();
    let mut templates_to_print: HashMap<usize, Vec<TemplateWithText>> = HashMap::new();
    for page in parser {
        let wikitext = &page.text;
        let output = configuration.parse(wikitext);
        if verbose {
            print_parser_warnings(&page, &output.warnings);
        }
        TemplateVisitor::new(wikitext).visit(&output.nodes, &mut |template, template_node| {
            let mut normalized_name = [0u8; MAX_TEMPLATE_NAME];
            if let Some(name) = normalize_template_name(template.name, &mut normalized_name) {
                if let Some((_file, file_number)) = template_to_file.get(name) {
                    let templates = templates_to_print.entry(*file_number)
                        .or_insert_with(|| Vec::new());
                    templates.push(TemplateWithText::new(
                        &wikitext[template_node.start()..template_node.end()],
                        template));
                }
                
            }
        });
        if templates_to_print.len() > 0 {
            for (file_number, templates) in templates_to_print.drain() {
                if let Some(writer) = file_number_to_file.get(&file_number) {
                    let title = page.title.to_string();
                    let mut writer = &mut *writer.borrow_mut();
                    let output = TemplatesInPage { title, templates };
                    match format {
                        SerializationFormat::JSON => {
                            serde_json::to_writer(&mut writer, &output).unwrap();
                            writeln!(&mut writer).unwrap();
                        },
                        SerializationFormat::CBOR => {
                            serde_cbor::to_writer(&mut writer, &output).unwrap();
                        },
                    }
                } else {
                    eprintln!("invalid file number {}", file_number);
                }
            }
        }
    }
    let parse_time = parse_start.elapsed();
    eprintln!("startup took {}, parsing and printing {}",
        print_time(&start_time),
        print_time(&parse_time)
    );
    
}

fn main() {
    let main_start = Instant::now();
    let opts = args::get_opts();
    let verbose = opts.verbose;
    match opts.cmd {
        CommandData::DumpTemplates { options: opts } => {
            let TemplateDumpOptions { files, dump_options: opts } = opts;
            let parser = parse_dump(opts.dump_file);
            let mut dumper = TemplateDumper::new(files);
            dumper.add_redirects();
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            dumper.parse(parser, opts.pages, opts.namespaces, verbose);
            let parse_time = parse_start.elapsed();
            eprintln!("startup took {}, parsing {}",
                print_time(&start_time),
                print_time(&parse_time)
            );
        },
        CommandData::DumpParsedTemplates { options: opts, format } => {
            dump_parsed_templates(opts, main_start, verbose, format);
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
                print_time(&parse_time)
            );
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
                print_time(&parse_time)
            );
        },
        CommandData::AddTemplateRedirects { files, suffix } => {
            for path in files {
                let mut template_names_and_files: HashMap<_, _> =
                    args::collect_template_names_and_files(&[&path])
                    .into_iter()
                    .map(|(template, filepath)| {
                        let filepath = filepath.unwrap_or_else(|| {
                            format!("{}{}", template, suffix)
                        });
                        (template, filepath)
                    })
                    .collect();
                template_dumper::add_template_redirects(&mut template_names_and_files);
                let mut template_names_and_files: Vec<_> = template_names_and_files
                    .into_iter()
                    .collect();
                template_names_and_files.sort_by(|(a, _), (b, _)| UniCase::new(a).cmp(&UniCase::new(b)));
                let mut path = path.into_os_string();
                let suffix: &OsStr = ".new".as_ref();
                path.push(suffix);
                let mut file = BufWriter::new(File::create(&path).unwrap());
                for (a, b) in template_names_and_files {
                    write!(file, "{}\t{}\n", a, b).unwrap();
                }
            }
        },
    }
}