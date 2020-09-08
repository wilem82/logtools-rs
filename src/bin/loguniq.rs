use std::io::BufRead;
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

    let entry_regex = regex::Regex::new(cli.value_of("entry-pattern").unwrap())?;
    let entries = LogEntryIterator::new(
        &entry_regex,
        None,
        Box::new(input.lines().filter_map(|x| match x {
            Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
            Ok(line) => Some(line),
        }))
    );

    //let started = std::time::Instant::now();

    let mut aggregated = std::collections::HashMap::<String, u64>::new();
    let replaces = vec![
        (regex::Regex::new(r"\d+")?, "<num>"),
    ];
    entries
        .map(|it| {
            let captured = entry_regex.captures(&it.text).unwrap();
            let timestamp = captured.name("timestamp").unwrap().as_str();
            let mut replaced = captured.name("message").unwrap().as_str().to_string();
            for pair in &replaces {
                replaced = pair.0.replace_all(replaced.as_str(), pair.1).to_string();
            }
            format!("{} {}\n", timestamp, replaced)
        })
        .for_each(|it| {
            *aggregated.entry(it).or_insert(0) += 1;
        });
    let inverse_sorted = {
        let mut vec: std::vec::Vec<(String, u64)> = aggregated.into_iter().map(|(line, count)| (line, count)).collect();
        vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        vec
    };

    for (line, count) in inverse_sorted.into_iter() {
        output.write(format!("{:8} {}", count, line).as_bytes()).expect("failed to write to output");
    }

    //println!("Elapsed: {:?}", started.elapsed());

    Ok(())
}

fn cli_app<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("loguniq")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Count similar log entries.")
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
}
