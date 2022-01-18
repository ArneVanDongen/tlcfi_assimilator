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
// TODO give the vlog file the correct name

/// The entry point for this program
///
/// Expects a file `tlcfi.txt` (or the one given in command args) with lines looking like s:
///
/// ```
/// 2021-10-15 16:07:42,994 INFO tlcFiMessages:41 - IN - {"jsonrpc":"2.0","method":"UpdateState","params":{"ticks":2181449574,"update":[{"objects":{"ids":["01"],"type":3},"states":[{"predictions":[{"likelyEnd":2181456774,"maxEnd":2181468174,"minEnd":2181456774,"state":6},{"likelyEnd":2181460774,"maxEnd":2181472174,"minEnd":2181460774,"state":8}]}}]}]}}
/// ```
///
/// The line is split in three parts using `- ` as a delimiter, and we assume the tlcfi json is the 3rd element. The second element is used to see whether a message is incoming or outgoing of ST.
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

    let first_tick = Option::None;

    let mut changes: Vec<TimestampedChanges> = Vec::new();

    let mut time_sorted_lines = Vec::new();
    for line in reader.lines() {
        if app_args.is_chronological {
            time_sorted_lines.push(line.unwrap());
        } else {
            time_sorted_lines.insert(0, line.unwrap());
        }
    }

    read_lines_and_save_changes(time_sorted_lines, first_tick, &mut changes);

    let vlog_messages = vlog_transformer::to_vlog(changes, app_args.start_date_time, app_args.vlog_tlcfi_mapping_file);

    let mut file = File::create("test.vlg").unwrap();
    for msg in vlog_messages {
        write!(file, "{}\r\n", msg).unwrap();
    }
}

fn read_lines_and_save_changes(time_sorted_lines: Vec<String>, mut first_tick: Option<u64>, changes: &mut Vec<TimestampedChanges>) {
    for line in time_sorted_lines {
        let filtered_line = line.replace("\"\"", "\"");
        let split_line: Vec<&str> = filtered_line.split("- ").collect();

        if split_line.len() != 3 {
            // This program is only familiar with lines that split into three parts with "- "
            println!("Following line was not able to be split by '- ' as expected: {}", line);
            continue;
        }

        // Only consider message from the TLC.
        if split_line[1].contains("IN") {
            if first_tick == Option::None {
                first_tick = tlcfi_parsing::find_first_tick(&split_line[2]);
            }
            if first_tick != Option::None {
                let timestamped_changes_res = tlcfi_parsing::parse_string(split_line[2], first_tick.unwrap());
                changes.extend(timestamped_changes_res.unwrap());
            }
        }
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
            .opt_value_from_fn("--tlcfi-log-file", check_file_existence)?
            .unwrap_or("tlcfi.txt".to_string()),
        vlog_tlcfi_mapping_file: pargs.free_from_fn(check_file_existence)?,
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

fn check_file_existence(file_name: &str) -> Result<String, String> {
    match File::open(file_name) {
        Ok(_) => Ok(file_name.to_string()),
        Err(error) => Err(format!("File name passed as argument '{}' could not be opened. Did you make a typo?\n{}", file_name, error)),
    }
} 

#[derive(Debug)]
struct AppArgs {
    is_chronological: bool,
    start_date_time: chrono::NaiveDateTime,
    tlcfi_log_file: String,
    vlog_tlcfi_mapping_file: String,
}
