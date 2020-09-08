use plotters::{prelude as pp};
use plotters::drawing::IntoDrawingArea;
use plotters::style::IntoFont;

use std::io::BufRead;

#[derive(Debug)]
struct Data {
    dt: chrono::DateTime<chrono::offset::Utc>,
    reqs: usize,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let clap = clap::App::new("accesslog-plot")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about("Generate a chart for requests per second")
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
            .help("Write SVG chart to this file")
        )
        .arg(clap::Arg::with_name("entry-pattern")
             .long("entry-pattern")
             .value_name("regex")
             .help("Regex capturing the first line of a log entry. Should have a named capture group `timestamp`.")
             .default_value(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3} .*\[ START TIME: (?P<timestamp>[^]]+)\]")
        )
        .arg(clap::Arg::with_name("timestamp-pattern")
             .long("timestamp-pattern")
             .value_name("pattern")
             .help("Pattern for parsing contents of the 'timestamp' regex capture group into a date/time structure. For syntax see Rust's chrono::format::strftime docs.")
             .default_value(r"%d/%m/%Y %H:%M:%S.%3f")
        )
        .arg(clap::Arg::with_name("chart-width")
            .long("chart-width")
            .short("w")
            .default_value("1024")
            .help("Width of the generated chart in pixels")
        )
        .arg(clap::Arg::with_name("chart-height")
            .long("chart-height")
            .short("h")
            .default_value("768")
            .help("Height of the generated chart in pixels")
        )
        .arg(clap::Arg::with_name("series-colour")
            .long("series-colour")
            .short("c")
            .default_value("red")
            .possible_values(&["red", "blue", "green"])
            .help("Colour of the series line")
        )
        .arg(clap::Arg::with_name("max-value")
            .long("max-value")
            .takes_value(true)
            .help("Instead of using the max value from the data, specify another value")
        )
        ;
    let cli = clap.get_matches();
    
    let output_filename = cli.value_of("output-file").unwrap();

    let data = {
        let input = std::io::BufReader::new(
            std::fs::File::open(
                cli.value_of("input-file").unwrap()
            ).expect("Can't open input file")
        );
        let entry_regex = regex::Regex::new(cli.value_of("entry-pattern").unwrap()).expect("Invalid regex for `entry-pattern`");
        let entries = logentry::entry::LogEntryIterator::new(
            &entry_regex,
            Some(cli.value_of("timestamp-pattern").unwrap()),
            Box::new(input.lines().filter_map(|it| match it {
                Err(e) => { eprintln!("ERROR: reading input: {}", e); None },
                Ok(line) => Some(line),
            }))
        );
        let mut out = Vec::new();
        for entry in entries {
            use chrono::Timelike;
            let reduced_dt = entry.zdt.unwrap().with_nanosecond(0).unwrap();

            if out.is_empty() {
                out.push(Data { dt: reduced_dt, reqs: 0 });
            }
 
            let mut last = out.last_mut().unwrap();
            if reduced_dt == last.dt {
                last.reqs += 1;
            } else {
                out.push(Data { dt: reduced_dt, reqs: 1 });
            }
        }
        out
    };

    let root = {
        let width = cli.value_of("chart-width").unwrap().parse::<u32>().expect("Invalid chart-width value");
        let width = width - (width % 8);
        let height = cli.value_of("chart-height").unwrap().parse::<u32>().expect("Invalid chart-height value");
        let height = height - (height % 8);
        println!("Chart is {}x{}", width, height);
        pp::SVGBackend::new(output_filename, (width, height)).into_drawing_area()
    };
    root.fill(&pp::WHITE)?;

    let dt_from = data.iter().map(|it| it.dt).min().unwrap();
    let dt_to = data.iter().map(|it| it.dt).max().unwrap();
    let caption = format!("{} - {}", dt_from, dt_to);
    let max_value = match cli.value_of("max-value") {
        Some(it) => it.parse::<usize>().unwrap(),
        None => data.iter().map(|it| it.reqs).max().unwrap(),
    };
    println!("X: {} to {}", dt_from, dt_to);
    println!("Y: {} to {}", 0, max_value);
    let mut chart = pp::ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(50)
        .y_label_area_size(50)
        .build_cartesian_2d(
            dt_from..dt_to,
            0..max_value,
        )?
        ;

    chart.configure_mesh().draw()?;

    let line_colour = match cli.value_of("series-colour").unwrap() {
        "red" => &pp::RED,
        "blue" => &pp::BLUE,
        "green" => &pp::GREEN,
        _ => panic!("unknown colour"),
    };
    chart
        .draw_series(pp::LineSeries::new(
            data.iter().map(|it| (it.dt, it.reqs)),
            line_colour,
        ))?
        ;

    Ok(())
}
