use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::fs::File;
use std::io::Write;

use either::{Left, Right};

use logentry::entry::*;
use logtools::matchers::parse_matchers;

fn main() {
    std::process::exit(match main0() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("ERROR: {:?}", e);
            1
        }
    });
}

fn main0<'a>() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli_app = cli_app();
    let cli = cli_app.get_matches();

    let include_matchers = parse_matchers(&cli, "verbatim-includes", "regex-includes");
    let exclude_matchers = parse_matchers(&cli, "verbatim-excludes", "regex-excludes");

    let stdout = std::io::stdout();
    let mut output = match cli.value_of("output-file") {
        Some(filename) => Left(BufWriter::new(File::create(filename)?)),
        None => Right(BufWriter::new(stdout.lock())),
    };

    let stdin = std::io::stdin();
    let input = match cli.value_of("input-file") {
        Some(filename) => Left(BufReader::new(File::open(filename)?)),
        None => Right(stdin.lock()),
    };

    let entry_regex_expr = {
        let entry_pattern = cli.value_of("entry-pattern").unwrap();
        match cli.is_present("skip-entry-source") {
            true => format!("{}{}", SKIP_ENTRY_SOURCE_REGEX.as_str(), entry_pattern),
            false => format!("^{}", entry_pattern),
        }
    };
    let entry_regex = regex::Regex::new(&entry_regex_expr)?;
    let entries = LogEntryIterator::new(
        &entry_regex,
        None,
        Box::new(input.lines().filter_map(|x| match x {
            Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
            Ok(line) => Some(line),
        }))
    );

    entries
        .filter(|it| include_matchers.is_empty() || include_matchers.iter().any(|m| m.matches(&it.text)))
        .filter(|it| !exclude_matchers.iter().any(|m| m.matches(&it.text)))
        .map(|mut it| {
            it.text.push('\n');
            it
        })
        .take_while(|it| {
            match output.write(it.text.as_bytes()) {
                Err(e) => { eprintln!("ERROR: writing output: {}", e); false },
                Ok(_) => true,
            }
        })
        .for_each(|_| ());

    Ok(())
}

lazy_static::lazy_static! {
    static ref SKIP_ENTRY_SOURCE_REGEX: String = "^[^:]+: ".to_string();
    static ref SKIP_ENTRY_SOURCE_HELP: String = format!(
        "A shortcut that prepends the entry-pattern regex with `{}` to be able to match log entries \
                produced by `logmerge -S` , i.e. to successfully match on the source log file indicator for each log entry.",
        SKIP_ENTRY_SOURCE_REGEX.as_str()
    );
}

fn cli_app<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("loggrep")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Grep that knows what a log entry is")
        .arg(clap::Arg::with_name("output-file")
            .long("output-file")
            .short("o")
            .takes_value(true)
            .value_name("file")
            .help("Write matching entries to that file (default is stdout)")
        )
        .arg(clap::Arg::with_name("input-file")
            .required(false)
            .index(1)
            .help("Read log entries from this file (default is stdin)")
        )
        .arg(clap::Arg::with_name("verbatim-includes")
             .long("verbatim-include")
             .short("f")
             .takes_value(true)
             .multiple(true)
             .number_of_values(1)
             .value_name("string")
             .help("Output log entries that match this string as is")
        )
        .arg(clap::Arg::with_name("regex-includes")
             .long("regex-include")
             .short("r")
             .takes_value(true)
             .multiple(true)
             .number_of_values(1)
             .value_name("regex")
             .help("Output log entries that match this regex")
        )
        .arg(clap::Arg::with_name("verbatim-excludes")
             .long("verbatim-exclude")
             .short("F")
             .takes_value(true)
             .multiple(true)
             .number_of_values(1)
             .value_name("string")
             .help("Don't output log entries that match this string as is")
        )
        .arg(clap::Arg::with_name("regex-excludes")
             .long("regex-exclude")
             .short("R")
             .takes_value(true)
             .multiple(true)
             .number_of_values(1)
             .value_name("regex")
             .help("Don't output log entries that match this regex")
        )
        .arg(clap::Arg::with_name("entry-pattern")
             .long("entry-pattern")
             .takes_value(true)
             .value_name("regex")
             .help("Regex matching the first line of a log entry. Automatically prepended with `^`.")
             .default_value(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3} ")
        )
        .arg(clap::Arg::with_name("skip-entry-source")
            .long("skip-entry-source")
            .short("L")
            .help(&SKIP_ENTRY_SOURCE_HELP)
        )
}
