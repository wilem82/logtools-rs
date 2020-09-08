use std::io::BufReader;
use std::io::BufWriter;
use std::fs::File;
use std::io::Write;

use either::{Left, Right};

use logentry::entry::*;
use logentry::multi::*;

fn main() {
    std::process::exit(match main0() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("ERROR: {:?}", e);
            1
        }
    });
}

static DEBUG_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn debug<F>(op: F)
where
    F: Fn() -> String
{
    if !DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    eprintln!("% {}", op());
}

fn main0<'a>() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli_app = cli_app();
    let cli = cli_app.get_matches();

    DEBUG_ENABLED.store(cli.is_present("print-debug"), std::sync::atomic::Ordering::Relaxed);

    let stdout = std::io::stdout();
    let mut output = match cli.value_of("output-file") {
        Some(filename) => Left(BufWriter::new(match File::create(filename) {
            Err(e) => fatal(format!("ERROR: creating file {}: {}", filename, e)),
            Ok(it) => it,
        })),
        None => Right(BufWriter::new(stdout.lock())),
    };

    let entry_regex = regex::Regex::new(cli.value_of("entry-pattern").unwrap())?;

    let include_glob = match cli.value_of("include-glob") {
        Some(it) => Some(globset::Glob::new(it)?.compile_matcher()),
        None => None
    };
    let exclude_glob = match cli.value_of("exclude-glob") {
        Some(it) => Some(globset::Glob::new(it)?.compile_matcher()),
        None => None
    };

    let timestamp_pattern = Some(cli.value_of("timestamp-pattern").unwrap());
    let truncate_last_dir = cli.is_present("truncate-last-dir");
    let no_source = cli.is_present("no-entry-source");

    let entry_iterators = walkdir::WalkDir::new(cli.value_of("directory").unwrap())
        .min_depth(1)
        .into_iter()
        .filter_map(|it| match it {
            Err(e) => { eprintln!("ERROR: {}", e); None },
            Ok(dirent) => Some(dirent),
        })
        .filter(|dirent| {
            if !dirent.file_type().is_file() {
                return false;
            }
            let filename = dirent.path();
            debug(|| format!("Going over {}", &filename.to_string_lossy()));
            if include_glob.as_ref().map_or(false, |it| !it.is_match(filename)) {
                debug(|| format!("Include glob didn't match, skipping file"));
                return false;
            }
            if exclude_glob.as_ref().map_or(false, |it| it.is_match(filename)) {
                debug(|| format!("Exclude glob matched, skipping file"));
                return false;
            }
            debug(|| format!("File matched"));
            true
        })
        .map(|dirent| {
            let path = dirent.path();
            let source = match truncate_last_dir {
                false => match no_source {
                    false => Some(path.to_str().unwrap().to_string()),
                    true => None,
                },
                true => {
                    Some(format!("{}{}{}",
                        match path.parent().unwrap().file_name() {
                            Some(it) => it.to_str().unwrap(),
                            None => ".",
                        },
                        std::path::MAIN_SEPARATOR,
                        path.file_name().unwrap().to_str().unwrap()
                    ))
                },
            };
            let file = match std::fs::File::open(path) {
                Err(e) => fatal(format!("ERROR: opening file {}: {}", path.to_string_lossy(), e)),
                Ok(it) => it,
            };
            let reader = BufReader::new(file);
            use bstr::io::BufReadExt;
            LogEntryIteratorWithSource::new(
                LogEntryIterator::new(
                    &entry_regex,
                    timestamp_pattern,
                    Box::new(reader.byte_lines().filter_map(|it| match it {
                        Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
                        Ok(line) => Some(String::from_utf8_lossy(line.as_ref()).to_string()),
                    }))
                ),
                source
            )
        })
        .collect::<Vec<LogEntryIteratorWithSource>>()
    ;

    MultiLogEntryIterator::new(entry_iterators)
        .for_each(|(entry, source)| {
            let text = match source {
                None => format!("{}\n", entry.text),
                Some(it) => format!("{}: {}\n", it, entry.text),
            };
            output.write(text.as_bytes()).expect("writing to output");
        });

    Ok(())
}

fn cli_app<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("logmerge")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Merge multiple log files into one keeping chronological order of the entries.")
        .arg(clap::Arg::with_name("directory")
            .required(true)
            .index(1)
            .help("Merge log files from that directory.")
        )
        .arg(clap::Arg::with_name("output-file")
            .long("output-file")
            .short("o")
            .takes_value(true)
            .value_name("file")
            .help("Write results to that file (default is stdout)")
        )
        .arg(clap::Arg::with_name("print-debug")
            .long("print-debug")
            .help("Enable printing of debug info to stderr.")
        )
        .arg(clap::Arg::with_name("entry-pattern")
             .long("entry-pattern")
             .value_name("regex")
             .help("Regex capturing the first line of a log entry. Must have a named capture group 'timestamp'.")
             .default_value(r"^(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3}) ")
        )
        .arg(clap::Arg::with_name("timestamp-pattern")
             .long("timestamp-pattern")
             .value_name("pattern")
             .help("Pattern for parsing contents of the 'timestamp' regex capture group into a date/time structure. For syntax see Rust's chrono::format::strftime docs.")
             .default_value(r"%Y-%m-%d %H:%M:%S,%3f")
        )
        .arg(clap::Arg::with_name("include-glob")
             .long("include-glob")
             .short("i")
             .value_name("glob")
             .help("Only merge files that match this glob")
             .default_value(r"*.{log,log.[0-9]*}")
        )
        .arg(clap::Arg::with_name("exclude-glob")
             .long("exclude-glob")
             .short("x")
             .takes_value(true)
             .value_name("glob")
             .help("Skip files that match this glob")
        )
        .arg(clap::Arg::with_name("truncate-last-dir")
             .long("truncate-last-dir")
             .short("S")
             .help("For output log entry source path, truncate to last directory entry")
        )
        .arg(clap::Arg::with_name("no-entry-source")
             .long("no-entry-source")
             .short("s")
             .conflicts_with("truncate-last-dir")
             .help("Don't prepend each log entry with its source log file path")
        )
}

fn fatal<S: Into<String>>(s: S) -> ! {
    eprintln!("{}", s.into());
    std::process::exit(1)
}
