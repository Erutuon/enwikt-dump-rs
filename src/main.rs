use dump_parser::{
    parse as parse_dump, parse_wiki_text::Positioned, Page, Warning,
};
use filter_headers::HeaderFilterer;
use header_stats::HeaderStats;
use serde::Serialize;
use smartstring::alias::String as SmartString;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    fmt::{Error as FmtError, Write as WriteFmt},
    fs::File,
    hash::{Hash, Hasher},
    io::{self, BufWriter, Result as IoResult, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use template_iter::{normalize_title, TemplateBorrowed, TemplateVisitor};

mod args;
use args::{
    Args, CommandData, DumpOptions, DumpParsedTemplates, SerializationFormat,
};

mod error;
use error::{Error, Result};

fn print_time(time: &Duration) -> std::result::Result<String, FmtError> {
    let mut secs = time.as_secs();
    let mins = secs / 60;
    let mut printed = String::new();
    if mins > 0 {
        secs %= 60;
        write!(printed, "{}m ", mins)?;
    }
    write!(printed, "{}.", secs)?;
    let decimals = format!("{:09}", time.subsec_nanos());
    printed.push_str({
        if secs == 0 && mins == 0 {
            let zero_count = decimals
                .as_bytes()
                .iter()
                .take_while(|&&b| b == b'0')
                .count();
            match zero_count {
                0..=2 => &decimals[..3],
                3..=5 => &decimals[..6],
                _ => &decimals[..9],
            }
        } else {
            &decimals[..3]
        }
    });
    printed.push_str("s");
    Ok(printed)
}

fn do_dumping<S>(dumper: &S, pretty: bool) -> Result<()>
where
    S: Serialize,
{
    if pretty {
        serde_json::to_writer_pretty(std::io::stdout().lock(), &dumper)?
    } else {
        serde_json::to_writer(std::io::stdout().lock(), &dumper)?
    }
    Ok(())
}

fn print_parser_warnings(page: &Page, warnings: &[Warning]) {
    for warning in warnings {
        let Warning {
            start,
            end,
            message,
        } = warning;
        let range = 0..page.text.len();
        let message = message.message().trim_end_matches('.');
        if !(range.contains(&start) && range.contains(&end)) {
            eprintln!("byte position {} or {} in warning {} is out of range of {:?}, size of [[{}]]",
                start, end, message, range, &page.title);
        } else {
            eprintln!(
                "{} at bytes {}..{} ({:?}) in [[{}]]",
                &message,
                start,
                end,
                &page.text[*start..*end],
                &page.title
            );
        }
    }
}

#[derive(Debug, Serialize)]
struct TemplateToDump {
    name: SmartString,
    parameters: BTreeMap<SmartString, SmartString>,
    text: Option<SmartString>,
}

impl<'a> TemplateToDump {
    fn new(
        wikitext: &'a str,
        template: TemplateBorrowed<'a>,
        with_text: bool,
    ) -> Self {
        let name = template.name.as_ref().into();
        let parameters = template
            .parameters
            .into_iter()
            .map(|(k, v)| (k.as_ref().into(), v.into()))
            .collect();
        let text = if with_text {
            Some(wikitext.into())
        } else {
            None
        };
        Self {
            name,
            parameters,
            text,
        }
    }
}

struct HashableWriter<W: Write> {
    id: usize,
    writer: W,
}

impl<W: Write> HashableWriter<W> {
    fn new(writer: W, id: usize) -> Self {
        Self { id, writer }
    }
}

impl<W: Write> Hash for HashableWriter<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<W: Write> PartialEq for HashableWriter<W> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<W: Write> Eq for HashableWriter<W> {}

impl<W: Write> Write for HashableWriter<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }
}

#[derive(Clone)]
struct ShareableHashableFile(Arc<Mutex<HashableWriter<BufWriter<File>>>>);

impl PartialEq for ShareableHashableFile {
    fn eq(&self, other: &Self) -> bool {
        self.0.lock().unwrap().eq(&other.0.lock().unwrap())
    }
}

impl Eq for ShareableHashableFile {}

// Cannot derive `Hash` because derive macro does not manage to delegate `Hash`
// to `HashableWriter`.
#[allow(clippy::derive_hash_xor_eq)]
impl Hash for ShareableHashableFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let writer: &HashableWriter<_> = &self.0.lock().unwrap();
        writer.hash(state);
    }
}

impl Write for ShareableHashableFile {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let mut file = self.0.lock().unwrap();
        file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        let mut file = self.0.lock().unwrap();
        file.flush()
    }
}

#[derive(Default)]
struct FilePool {
    files: HashMap<String, ShareableHashableFile>,
    count: usize,
}

impl FilePool {
    fn new() -> Self {
        Default::default()
    }

    fn create(&mut self, path: &str) -> IoResult<ShareableHashableFile> {
        match self.files.get(path) {
            Some(f) => Ok((*f).clone()),
            None => {
                let file = File::create(path)?;
                let file = BufWriter::new(file);
                let file_ref = ShareableHashableFile(Arc::new(Mutex::new(
                    HashableWriter::new(file, self.get_file_id()),
                )));
                let cloned = file_ref.clone();
                self.files.insert(path.to_string(), file_ref);
                Ok(cloned)
            }
        }
    }

    fn get_file_id(&mut self) -> usize {
        let count = self.count;
        self.count += 1;
        count
    }
}

#[derive(Debug, Serialize)]
struct TemplatesInPage<'a> {
    title: &'a str,
    templates: &'a [TemplateToDump],
}

fn dump_parsed_templates(
    options: DumpParsedTemplates,
    main_start: Instant,
    verbose: bool,
) -> Result<()> {
    dbg!(&options.dump_options);
    let DumpParsedTemplates {
        format,
        files: template_to_file,
        template_normalizations,
        include_text,
        dump_options:
            DumpOptions {
                pages,
                namespaces,
                dump_file,
            },
    } = options;
    let template_normalizations_ref = template_normalizations.as_ref();
    let parser = parse_dump(dump_file).take(pages);
    let mut files = FilePool::new();
    let extension = match format {
        SerializationFormat::Cbor => ".cbor",
        SerializationFormat::Json => ".jsonl",
    };
    let template_to_file = template_to_file
        .into_iter()
        .map(|(template, path)| {
            let normalized = normalize_title(&template).map_err(|e| {
                Error::TemplateNameNormalization {
                    title: template,
                    cause: e,
                }
            })?;
            let path = path.unwrap_or_else(|| normalized.clone() + extension);
            let file = files.create(&path).map_err(|e| Error::IoError {
                action: "create",
                path: path.into(),
                cause: e,
            })?;
            Ok((normalized, file))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    let (sender, receiver) = crossbeam_channel::unbounded::<(
        SmartString,
        HashMap<ShareableHashableFile, Vec<TemplateToDump>>,
    )>();
    eprintln!("starting print thread");
    let print_thread = thread::spawn(move || -> Result<()> {
        let mut i = 0;
        while let Ok((title, templates_to_print)) = receiver.recv() {
            i += 1;
            eprintln!("{}: {}", i, title);
            for (file, templates) in templates_to_print {
                let output = TemplatesInPage {
                    title: &title,
                    templates: &templates,
                };
                let mut writer = file;
                match format {
                    SerializationFormat::Json => {
                        serde_json::to_writer(&mut writer, &output)?;
                        write!(&mut writer, "\n").unwrap();
                    }
                    SerializationFormat::Cbor => {
                        serde_cbor::to_writer(&mut writer, &output)?;
                    }
                }
            }
        }
        Ok(())
    });
    let configuration = dump_parser::wiktionary_configuration();
    let start_time = main_start.elapsed();
    let parse_start = Instant::now();
    eprintln!("starting parsing loop");
    for page in parser {
        let page = page?;
        dbg!(&page.title);
        if !namespaces.contains(
            &page
                .namespace
                .try_into()
                .map_err(|_| Error::NamespaceConversionError(page.namespace))?,
        ) {
            continue;
        }
        let mut templates_to_print = HashMap::new();
        let wikitext = &page.text;
        let output = configuration.parse(wikitext);
        if verbose {
            print_parser_warnings(&page, &output.warnings);
        }
        let visitor = TemplateVisitor::new(wikitext);
        visitor.visit(&output.nodes, &mut |mut template, template_node| {
            if let Ok(name) = normalize_title(&template.name) {
                if let Some(file) = template_to_file.get(&name) {
                    if let Some(normalizations) = template_normalizations_ref {
                        template.name = normalizations
                            .get(&name)
                            .map(|normalized| {
                                Cow::Borrowed(normalized.as_ref())
                            })
                            .unwrap_or_else(|| Cow::Owned(name));
                    }
                    let templates = templates_to_print
                        .entry(file.clone())
                        .or_insert_with(Vec::new);
                    templates.push(TemplateToDump::new(
                        template_node.get_text_from(&wikitext),
                        template,
                        include_text,
                    ));
                }
            }
        });
        sender
            .send((page.title.into(), templates_to_print))
            .unwrap();
    }
    eprintln!("finished parsing loop");
    drop(sender);
    print_thread.join().unwrap()?;
    eprintln!("finished print thread");
    let parse_time = parse_start.elapsed();
    eprintln!(
        "startup took {}, parsing and printing {}",
        print_time(&start_time).unwrap(),
        print_time(&parse_time).unwrap()
    );
    Ok(())
}

fn try_main() -> Result<()> {
    let main_start = Instant::now();
    let opts = args::get_opts()?;
    let verbose = opts.verbose;
    match opts.cmd {
        CommandData::DumpParsedTemplates(options) => {
            dump_parsed_templates(options, main_start, verbose)?;
        }
        CommandData::AllHeaders {
            pretty,
            dump_options: opts,
        } => {
            let parser = parse_dump(opts.dump_file);
            let mut dumper = HeaderStats::new();
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            dumper.parse(parser, opts.pages, opts.namespaces, verbose);
            do_dumping(&dumper, pretty).unwrap_or_else(|e| eprintln!("{}", e));
            let parse_time = parse_start.elapsed();
            eprintln!(
                "startup took {}, parsing and printing {}",
                print_time(&start_time).unwrap(),
                print_time(&parse_time).unwrap()
            );
        }
        CommandData::FilterHeaders {
            top_level_headers,
            other_headers,
            pretty,
            dump_options: opts,
        } => {
            let parser = parse_dump(opts.dump_file);
            let mut filterer =
                HeaderFilterer::new(top_level_headers, other_headers);
            let start_time = main_start.elapsed();
            let parse_start = Instant::now();
            filterer.parse(parser, opts.pages, opts.namespaces, verbose);
            do_dumping(&filterer, pretty)?;
            let parse_time = parse_start.elapsed();
            eprintln!(
                "startup took {}, parsing and printing {}",
                print_time(&start_time).unwrap(),
                print_time(&parse_time).unwrap()
            );
        }
        CommandData::Completions { shell } => {
            Args::clap().gen_completions_to(
                env!("CARGO_PKG_NAME"),
                shell,
                &mut io::stdout(),
            );
        }
    }
    Ok(())
}

fn main() {
    try_main().unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
}
