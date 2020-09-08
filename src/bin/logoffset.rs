use std::io::BufReader;
use std::io::BufWriter;
use std::fs::File;
use std::io::Write;

use either::{Left, Right};

use logentry::entry::*;

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

    let timestamp_pattern = Some(cli.value_of("timestamp-pattern").unwrap());

    let entry_regex = regex::Regex::new(cli.value_of("entry-pattern").unwrap())?;
    use bstr::io::BufReadExt;
    let entries = LogEntryIterator::new(
        &entry_regex,
        timestamp_pattern,
        Box::new(input.byte_lines().filter_map(|x| match x {
            Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
            Ok(line) => Some(String::from_utf8_lossy(line.as_ref()).to_string()),
        }))
    );

    let offset_text = cli.value_of("offset").unwrap();
    let offset_duration= match offset_text.parse::<i64>() {
        Err(e) => {
            eprintln!("ERROR: the specified offset `{}` is not an integer: {}", offset_text, e);
            std::process::exit(1);
        },
        Ok(it) => chrono::Duration::hours(it),
    };
    entries
        .for_each(|it| {
            let offset_zdt: Zdt = it.zdt.unwrap() + offset_duration;
            let captured_timestamp = entry_regex.captures(&it.text).unwrap().name("timestamp").unwrap().as_str();
            let new_timestamp = offset_zdt.format(timestamp_pattern.unwrap()).to_string();
            let new_text = it.text.replace(captured_timestamp, new_timestamp.as_str());
            output.write(format!(
                "{}\n",
                new_text
            ).as_bytes()).unwrap();
        })
    ;

    Ok(())
}

fn cli_app<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("logoffset")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Offset the timestamp of every log entry by the specified amount of hours")
        .arg(clap::Arg::with_name("output-file")
            .long("output-file")
            .short("o")
            .takes_value(true)
            .value_name("file")
            .help("Write results to that file (default is stdout).")
        )
        .arg(clap::Arg::with_name("input-file")
            .required(false)
            .index(1)
            .help("Read log entries from this file (default is stdin).")
        )
        .arg(clap::Arg::with_name("entry-pattern")
             .long("entry-pattern")
             .takes_value(true)
             .value_name("regex")
             .help("Regex matching the first line of a log entry. Must have `timestamp` and `message` capturing groups.")
             .default_value(r"^(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3}) (?P<message>.*)")
        )
        .arg(clap::Arg::with_name("offset")
            .long("offset-hours")
            .takes_value(true)
            .value_name("integer")
            .help("The offset represented by a signed integer, in hours, e.g. +1 or -3")
            .required(true)
        )
        .arg(clap::Arg::with_name("timestamp-pattern")
            .long("timestamp-pattern")
            .value_name("pattern")
            .help("Pattern for parsing contents of the 'timestamp' regex capture group into a date/time structure. For syntax see Rust's chrono::format::strftime docs.")
            .default_value(r"%Y-%m-%d %H:%M:%S,%3f")
        )
}
