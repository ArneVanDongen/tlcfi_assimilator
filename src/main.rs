use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

mod vlog_transformer;
mod tlcfi_parsing;

use tlcfi_assimilator::TimestampedChanges;

const ARGS_HELP: &str = "\
TLC-FI Assimilator

USAGE:
  tlcfi_assimilator [OPTIONS] --start-date-time STRING [VLOG_TLCFI_MAPPING_FILE]

FLAGS:
  -h, --help                Prints help information

OPTIONS:
  --chronological BOOL      Sets whether the logs are in chronological order (newest last) [default: false]
  --start-date-time STRING  ISO 8601 timestamp for the start moment of tlcfi logs (e.g. 2021-12-15T11:00:00.000)
  --tlcfi-log-file STRING   Sets the name of the file to load [default: tlcfi.txt]

ARGS:
  <VLOG_TLCFI_MAPPING_FILE>
";

// TODO handle all unwraps properly
// TODO extract tlcfi parsing to a different module
// TODO retrieve the start time of the logs

/// The entry point for this program
///
/// Expects a file `tlcfi.txt` with lines looking like the following:
///
/// ```
/// 2021-10-15 16:07:42,994 INFO tlcFiMessages:41 - IN - {"jsonrpc":"2.0","method":"UpdateState","params":{"ticks":2181449574,"update":[{"objects":{"ids":["01","04","05","06","12"],"type":3},"states":[{"predictions":[{"likelyEnd":2181456774,"maxEnd":2181468174,"minEnd":2181456774,"state":6},{"likelyEnd":2181460774,"maxEnd":2181472174,"minEnd":2181460774,"state":8}]},{"predictions":[{"confidence":15,"likelyEnd":2181450174,"maxEnd":2181450574,"minEnd":2181450174,"state":3},{"likelyEnd":2181456174,"maxEnd":2181535574,"minEnd":2181456174,"state":6},{"likelyEnd":2181460174,"maxEnd":2181539574,"minEnd":2181460174,"state":8}]},{"predictions":[{"confidence":3,"minEnd":2181449674,"state":3}]},{"predictions":[{"likelyEnd":2181455574,"maxEnd":2181529974,"minEnd":2181455574,"state":6},{"likelyEnd":2181459574,"maxEnd":2181533974,"minEnd":2181459574,"state":8}]},{"predictions":[{"confidence":3,"minEnd":2181449674,"state":3}]}]}]}}
/// ```
///
/// Looking from the end of the line, it is split in three parts using `-` as a delimiter.
fn main() {
    let app_args = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            print!("{}", ARGS_HELP);
            std::process::exit(1);
        }
    };

    let tlcfi_log_file = File::open(app_args.tlcfi_log_file).unwrap();
    let reader = BufReader::new(tlcfi_log_file);

    let mut first_tick = Option::None;

    let mut signal_changes: Vec<TimestampedChanges> = Vec::new();

    // Read the file line by line using the lines() iterator from std::io::BufRead.
    let mut time_sorted_lines = Vec::new();
    for line in reader.lines() {
        if app_args.is_chronological {
            time_sorted_lines.push(line.unwrap());
        } else {
            time_sorted_lines.insert(0, line.unwrap());
        }
    }

    for line in time_sorted_lines {
        // println!("Looking at line: {:?}", line);
        let filtered_line = line.replace("\"\"", "\"");
        let split_line: Vec<&str> = filtered_line.split("- ").collect();

        // Only consider message from the TLC.
        if split_line[1].contains("IN") {
            if first_tick == Option::None {
                first_tick = tlcfi_parsing::tlcfi_parsing::find_first_tick(&split_line[2]);
            }
            if first_tick != Option::None {
                let timestamped_changes_res = tlcfi_parsing::tlcfi_parsing::parse_string(split_line[2], first_tick.unwrap());
                signal_changes.extend(timestamped_changes_res.unwrap());
            }
        }
    }

    let vlog_messages = vlog_transformer::vlog_transformer::to_vlog(signal_changes, app_args.start_date_time);

    let mut file = File::create("test.vlg").unwrap();
    for msg in vlog_messages {
        write!(file, "{}\r\n", msg).unwrap();
    }
}

fn parse_args() -> Result<AppArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", ARGS_HELP);
        std::process::exit(0);
    }

    let args = AppArgs {
        is_chronological: pargs
            .opt_value_from_str("--chronological")?
            .unwrap_or(false),
        start_date_time: pargs.value_from_fn("--start-date-time", parse_date_time)?,
        tlcfi_log_file: pargs
            .opt_value_from_str("--tlcfi-log-file")?
            .unwrap_or("tlcfi.txt".to_string()),
        vlog_tlcfi_mapping_file: pargs.free_from_str()?,
    };

    Ok(args)
}

fn parse_date_time(arg: &str) -> Result<chrono::NaiveDateTime, String> {
    // <Year-month-day format (ISO 8601). Same to %Y-%m-%d><T><Hour-minute-second format. Same to %H:%M:%S><Similar to .%f but left-aligned. These all consume the leading dot.>
    match chrono::NaiveDateTime::parse_from_str(arg, "%FT%T%.3f") {
        Ok(date_time) => Ok(date_time),
        Err(error) => Err(format!("Failed to transform argument {} into a date time:\n{}", arg, error)),
    }
}


#[derive(Debug)]
struct AppArgs {
    is_chronological: bool,
    start_date_time: chrono::NaiveDateTime,
    tlcfi_log_file: String,
    vlog_tlcfi_mapping_file: String,
}
