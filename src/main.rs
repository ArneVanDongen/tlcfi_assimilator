use std::{
    fs::{File, read_to_string},
    io::{BufRead, BufReader, Write},
};

mod tlcfi_parsing;
mod vlog_transformer;

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

    run_with_args(app_args);
}

fn run_with_args(app_args: AppArgs) {
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
    let vlog_messages = vlog_transformer::to_vlog(
        changes,
        app_args.start_date_time,
        app_args.vlog_tlcfi_mapping_file,
    );
    let mut file = File::create("test.vlg").unwrap();
    for msg in vlog_messages {
        write!(file, "{}\r\n", msg).unwrap();
    }
}

fn read_lines_and_save_changes(
    time_sorted_lines: Vec<String>,
    mut first_tick: Option<u64>,
    changes: &mut Vec<TimestampedChanges>,
) {
    for line in time_sorted_lines {
        let filtered_line = line.replace("\"\"", "\"");
        let split_line: Vec<&str> = filtered_line.split("- ").collect();

        if split_line.len() != 3 {
            // This program is only familiar with lines that split into three parts with "- "
            println!(
                "Following line was not able to be split by '- ' as expected: {}",
                line
            );
            continue;
        }

        // Only consider message from the TLC.
        if split_line[1].contains("IN") {
            if first_tick == Option::None {
                first_tick = tlcfi_parsing::find_first_tick(&split_line[2]);
            }
            if first_tick != Option::None {
                let timestamped_changes_res =
                    tlcfi_parsing::parse_string(split_line[2], first_tick.unwrap());
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
        Err(error) => Err(format!(
            "Failed to transform argument {} into a date time:\n{}",
            arg, error
        )),
    }
}

fn check_file_existence(file_name: &str) -> Result<String, String> {
    match File::open(file_name) {
        Ok(_) => Ok(file_name.to_string()),
        Err(error) => Err(format!(
            "File name passed as argument '{}' could not be opened. Did you make a typo?\n{}",
            file_name, error
        )),
    }
}

#[derive(Debug)]
struct AppArgs {
    is_chronological: bool,
    start_date_time: chrono::NaiveDateTime,
    tlcfi_log_file: String,
    vlog_tlcfi_mapping_file: String,
}

#[cfg(test)]
mod test {

    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use tlcfi_assimilator::{self, DetectorState};

    const RELATIVE_TLCFI_FILE_PATH: &str = "./tlcfi.txt";
    const RELATIVE_VLOG_MAPPING_FILE_PATH: &str = "./vlog_tlcfi_mapping.txt";

    fn get_test_start_time() -> NaiveDateTime {
        NaiveDate::from_ymd(2021, 12, 15).and_hms(11, 00, 00)
    }

    #[test]
    fn checking_file_existence_of_valid_file_should_return_ok() {
        let file_existence = check_file_existence(RELATIVE_TLCFI_FILE_PATH);
        assert!(file_existence.is_ok());
    }

    #[test]
    fn checking_file_existence_of_invalid_file_should_return_err() {
        let file_existence = check_file_existence("./thisfiledoesnot.exist");
        assert!(file_existence.is_err());
    }

    #[test]
    fn parse_date_time_of_valid_iso_8601_str_should_return_ok() {
        let valid_time_stamp = "2021-12-15T11:00:00.000";
        let parsed_date_time = parse_date_time(valid_time_stamp);
        assert_eq!(parsed_date_time, Ok(get_test_start_time()));
    }

    #[test]
    fn parse_date_time_of_invalid_iso_8601_str_should_return_err() {
        assert!(parse_date_time("2021-12-15T11:00:00,000").is_err());
        assert!(parse_date_time("2021-12-15 11:00:00.000").is_err());
    }

    #[test]
    fn reading_an_empty_line_should_not_result_in_any_changes_added() {
        let lines = vec![String::from("")];
        let first_tick = Option::Some(0);
        let mut empty_changes = Vec::new();

        read_lines_and_save_changes(lines, first_tick, &mut empty_changes);

        assert!(empty_changes.is_empty());
    }

    #[test]
    fn reading_an_out_line_should_not_result_in_any_changes_added() {
        let lines = vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - OUT - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087974612,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")];
        let first_tick = Option::Some(0);
        let mut empty_changes = Vec::new();

        read_lines_and_save_changes(lines, first_tick, &mut empty_changes);

        assert!(empty_changes.is_empty());
    }

    #[test]
    fn reading_a_valid_line_should_mutate_changes() {
        let lines = vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - IN - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")];
        let first_tick = Option::Some(4000);
        let mut changes = Vec::new();

        read_lines_and_save_changes(lines, first_tick, &mut changes);

        assert!(!changes.is_empty());
        let change = &changes[0];
        assert_eq!(change.ms_from_beginning, 4087 - first_tick.unwrap());
        assert_eq!(change.detector_names[0], "D681");
        assert_eq!(change.detector_states[0], DetectorState::FREE);
        println!("{:?}", changes[0])
    }

    #[test]
    fn reading_a_valid_line_without_a_first_tick_should_set_the_first_tick_to_the_one_of_the_message() {
        let lines = vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - IN - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087974612,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")];
        let first_tick = Option::None;
        let mut changes = Vec::new();

        read_lines_and_save_changes(lines, first_tick, &mut changes);

        assert!(!changes.is_empty());
        assert_eq!(changes[0].ms_from_beginning, 0); // it being 0 means this is the very first message handled, and first tick is equal to it
    }

    /// Uses input files ./tlcfi.txt and ./vlog_tlcfi_mapping.txt for an integration test, and compares it with an expected vlog output: ./expected_vlog_output.vlg
    #[test]
    fn integration_test() {
        let expected_vlog_output = read_to_string("./expected_vlog_output.vlg").unwrap();
        let app_args = AppArgs {
            is_chronological: false,
            start_date_time: get_test_start_time(),
            tlcfi_log_file: RELATIVE_TLCFI_FILE_PATH.to_string(),
            vlog_tlcfi_mapping_file: RELATIVE_VLOG_MAPPING_FILE_PATH.to_string(),
        };

        run_with_args(app_args);
        let actual_vlog_output =  read_to_string("./test.vlg").unwrap();

        let mut expected_lines = Vec::new();
        expected_lines.extend(expected_vlog_output.split_whitespace().into_iter());
        for (i, actual_line) in actual_vlog_output.split_whitespace().enumerate() {
            assert_eq!(actual_line, expected_lines[i]);
            println!("Compared actual line {:?}, with expected line {:?}", actual_line, expected_lines[i]);
        }
    }
}
