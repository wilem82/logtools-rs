#[derive(serde::Serialize, serde::Deserialize, Clone, Eq)]
struct Entry {
    text: String,
    zdt: logentry::entry::Zdt,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.zdt.cmp(&other.zdt)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.zdt == other.zdt
    }
}

impl external_sort::ExternallySortable for Entry {
    fn get_size(&self) -> u64 {
        self.text.len() as u64
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let clap = clap::App::new("logsort")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Sort arbitrary-sized log file")
        .arg(clap::Arg::with_name("input-file")
            .required(true)
            .index(1)
            .help("Read log entries from this file")
        )
        .arg(clap::Arg::with_name("output-file")
            .required(true)
            .long("output-file")
            .short("o")
            .takes_value(true)
            .value_name("file")
            .help("Write sorted log entries into this file")
        )
        .arg(clap::Arg::with_name("entry-pattern")
             .long("entry-pattern")
             .value_name("regex")
             .help("Regex capturing the first line of a log entry. Must have a named capture group `timestamp`.")
             .default_value(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3} .*\[ START TIME: (?P<timestamp>[^]]+)\]")
        )
        .arg(clap::Arg::with_name("timestamp-pattern")
             .long("timestamp-pattern")
             .value_name("pattern")
             .help("Pattern for parsing contents of the 'timestamp' regex capture group into a date/time structure. For syntax see Rust's chrono::format::strftime docs.")
             .default_value(r"%d/%m/%Y %H:%M:%S.%3f")
        )
        ;
    let cli = clap.get_matches();
            
    let input = std::io::BufReader::new(
        std::fs::File::open(
            cli.value_of("input-file").unwrap()
        ).expect("Can't open input file")
    );
    let output_filename = cli.value_of("output-file").unwrap();
    let mut output = std::io::BufWriter::new(match std::fs::File::create(output_filename) {
        Err(e) => panic!("Can't open output file `{}` for writing: {}", output_filename, e),
        Ok(it) => it,
    });
    
    let entry_regex = regex::Regex::new(cli.value_of("entry-pattern").unwrap()).expect("Invalid regex for `entry-pattern`");
    use std::io::BufRead;
    let entries = logentry::entry::LogEntryIterator::new(
        &entry_regex,
        Some(cli.value_of("timestamp-pattern").unwrap()),
        Box::new(input.lines().filter_map(|it| match it {
            Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
            Ok(line) => Some(line),
        }))
    );
    let entries = entries.map(|it| Entry { text: it.text, zdt: it.zdt.unwrap() });
    let sorter = external_sort::ExternalSorter::new(1024 * 1024, None);
    let sorted_iter = sorter.sort(entries).unwrap();

    use std::io::Write;
    sorted_iter.for_each(|entry| {
        output.write(entry.unwrap().text.as_bytes()).expect("writing to output");
        output.write("\n".as_bytes()).expect("writing to output");
    });

    Ok(())
}
