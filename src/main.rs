use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

mod tlcfi_parsing;
mod vlog_transformer;

use chrono::NaiveDateTime;
use tlcfi_assimilator::AssimilationData;

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

    run_with_args(app_args)
}

fn run_with_args(app_args: AppArgs) {
    let time_sorted_lines = sort_lines(&app_args.tlcfi_log_file, &app_args.is_chronological);

    let start_time = &app_args.start_date_time.unwrap_or_else(||
        get_start_date_time_from_file(&time_sorted_lines).expect("Failed to get start date time from logs, set it in application arguments instead or filter the logs."));

    let mut data = AssimilationData {
        start_time: *start_time,
        sorted_lines: time_sorted_lines,
        first_tick: Option::None,
        changes: Vec::new(),
    };

    read_lines_and_save_changes(&mut data);
    let tlc_name =
        vlog_transformer::load_tlc_name(&app_args.vlog_tlcfi_mapping_file).expect(&format!(
            "Couldn't find a TLC name in the given VLog TLC FI mapping file: {:?}",
            &app_args.vlog_tlcfi_mapping_file
        ));

    let vlog_messages = vlog_transformer::to_vlog(
        data.changes,
        start_time,
        &app_args.vlog_tlcfi_mapping_file,
        &tlc_name,
    );

    let file_name = create_file_name(&tlc_name, start_time);

    let mut file = File::create(&file_name).expect(&format!(
        "Failed to create the file '{}' for saving the VLog output.",
        &file_name
    ));
    println!("Created file: {}", &file_name);

    for msg in vlog_messages {
        write!(file, "{}\r\n", msg).expect(&format!(
            "Failed to write line {:?} to the VLog output file.",
            msg
        ));
    }
}

fn sort_lines(tlcfi_log_file: &str, is_chronological: &bool) -> Vec<String> {
    let tlcfi_log_file =
        File::open(tlcfi_log_file).expect("Couldn't open the given file path for the TLC FI logs");
    let reader = BufReader::new(tlcfi_log_file);
    let mut time_sorted_lines = Vec::new();
    for line_res in reader.lines() {
        if let Ok(line) = line_res {
            if *is_chronological {
                time_sorted_lines.push(line);
            } else {
                time_sorted_lines.insert(0, line);
            }
        } else {
            eprintln!("Failed to read line {:?}", line_res)
        }
    }
    time_sorted_lines
}

fn read_lines_and_save_changes(data: &mut AssimilationData) {
    for line in &data.sorted_lines {
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
            if data.first_tick == Option::None {
                data.first_tick = tlcfi_parsing::find_first_tick(&split_line[2]);
            }
            if let Some(first_tick) = data.first_tick {
                if let Ok(timestamped_changes_res) =
                    tlcfi_parsing::parse_string(split_line[2], first_tick)
                {
                    data.changes.extend(timestamped_changes_res)
                } else {
                    eprintln!(
                        "Failed to parse string {:?} with first tick {:?}",
                        split_line[2], first_tick
                    )
                }
            } else {
                eprintln!("Didn't find a first tick yet!")
            }
        }
    }
}

fn create_file_name(tlc_name: &str, start_date_time: &NaiveDateTime) -> String {
    let date_part = start_date_time.date().to_string().replace("-", "");
    let time_part = &start_date_time.time().to_string().replace(":", "")[0..6];
    String::from(tlc_name) + "_" + &date_part + "_" + &time_part + ".vlg"
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
        start_date_time: pargs.opt_value_from_fn("--start-date-time", parse_date_time)?,
        tlcfi_log_file: pargs
            .opt_value_from_fn("--tlcfi-log-file", check_file_existence)?
            .unwrap_or("tlcfi.txt".to_string()),
        vlog_tlcfi_mapping_file: pargs.free_from_fn(check_file_existence)?,
    };
    Ok(args)
}

fn parse_date_time(arg: &str) -> Result<NaiveDateTime, String> {
    // <Year-month-day format (ISO 8601). Same to %Y-%m-%d><T><Hour-minute-second format. Same to %H:%M:%S><Similar to .%f but left-aligned. These all consume the leading dot.>
    match NaiveDateTime::parse_from_str(arg, "%FT%T%.3f") {
        Ok(date_time) => Ok(date_time),
        Err(error) => Err(format!(
            "Failed to transform argument {} into a date time:\n{}",
            arg, error
        )),
    }
}

fn get_start_date_time_from_file(
    sorted_lines: &Vec<String>,
) -> Result<NaiveDateTime, pico_args::Error> {
    let mut date_time_bit = String::new();
    for line in sorted_lines {
        let split_line: Vec<&str> = line.split("- ").collect();

        // if it is a logline
        if line.len() >= 23 && split_line.len() == 3 {
            date_time_bit = line[0..23].to_string();
            break;
        }
    }

    date_time_bit = date_time_bit.replace(",", ".").replace(" ", "T");

    match parse_date_time(&date_time_bit) {
        Ok(date_time) => Ok(date_time),
        Err(error) => Err(pico_args::Error::ArgumentParsingFailed {
            cause: format!("--start-date-time wasn't given and we couldn't extract it from the log file. Failed with error: {}", error),
        }),
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
    start_date_time: Option<NaiveDateTime>,
    tlcfi_log_file: String,
    vlog_tlcfi_mapping_file: String,
}

#[cfg(test)]
mod test {

    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::fs::read_to_string;
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
        let mut data = AssimilationData {
            start_time: get_test_start_time(),
            sorted_lines: vec![String::from("")],
            first_tick: Option::Some(0),
            changes: Vec::new(),
        };

        read_lines_and_save_changes(&mut data);

        assert!(data.changes.is_empty());
    }

    #[test]
    fn reading_an_out_line_should_not_result_in_any_changes_added() {
        let mut data = AssimilationData {
            start_time: get_test_start_time(),
            sorted_lines: vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - OUT - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087974612,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")],
            first_tick: Option::Some(0),
            changes: Vec::new(),
        };

        read_lines_and_save_changes(&mut data);

        assert!(data.changes.is_empty());
    }

    #[test]
    fn reading_a_valid_line_should_mutate_changes() {
        let mut data = AssimilationData {
            start_time: get_test_start_time(),
            sorted_lines: vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - IN - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")],
            first_tick: Option::Some(4000),
            changes: Vec::new(),
        };

        read_lines_and_save_changes(&mut data);

        assert!(!data.changes.is_empty());
        let change = &data.changes[0];
        assert_eq!(change.ms_from_beginning, 4087 - data.first_tick.unwrap());
        assert_eq!(change.detector_names[0], "D681");
        assert_eq!(change.detector_states[0], DetectorState::FREE);
    }

    #[test]
    fn reading_a_valid_line_without_a_first_tick_should_set_the_first_tick_to_the_one_of_the_message(
    ) {
        let mut data = AssimilationData {
            start_time: get_test_start_time(),
            sorted_lines: vec![String::from("2021-12-15 12:59:59,794 INFO  tlcFiMessages:41 - IN - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087974612,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")],
            first_tick: Option::None,
            changes: Vec::new(),
        };

        read_lines_and_save_changes(&mut data);

        assert!(!data.changes.is_empty());
        assert_eq!(data.changes[0].ms_from_beginning, 0); // it being 0 means this is the very first message handled, and first tick is equal to it
    }

    #[test]
    fn creating_a_vlog_file_name_should_use_the_tlc_name_and_format_the_date_time_correctly() {
        let tlc_name = "test";
        let date_time = get_test_start_time();

        let vlog_file_name = create_file_name(tlc_name, &date_time);

        assert_eq!(vlog_file_name, "test_20211215_110000.vlg");
    }

    #[test]
    fn creating_a_vlog_file_name_should_remove_ms_from_time() {
        let tlc_name = "test";
        let date_time = NaiveDate::from_ymd(2021, 12, 15).and_hms_milli(11, 22, 33, 444);

        let vlog_file_name = create_file_name(tlc_name, &date_time);

        assert_eq!(vlog_file_name, "test_20211215_112233.vlg");
    }

    #[test]
    fn getting_start_date_time_from_a_file_should_interpret_log_timestamps_as_naivedatetime() {
        let expected_start_date_time = parse_date_time("2021-12-15T11:00:00.074").unwrap();
        let lines = vec![String::from("2021-12-15 11:00:00,074 INFO  tlcFiMessages:41 - OUT - {\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087974612,\"update\":[{\"objects\":{\"ids\":[\"D681\"],\"type\":4},\"states\":[{\"state\":0}]}]}}")];

        let start_date_time_from_log = get_start_date_time_from_file(&lines);

        assert!(start_date_time_from_log.is_ok());
        assert_eq!(start_date_time_from_log.unwrap(), expected_start_date_time);
    }

    /// Uses input files ./tlcfi.txt and ./vlog_tlcfi_mapping.txt for an integration test, and compares it with an expected vlog output: ./expected_vlog_output.vlg
    #[test]
    fn integration_test() {
        let expected_vlog_output = read_to_string("./expected_vlog_output.vlg").unwrap();
        let app_args = AppArgs {
            is_chronological: false,
            start_date_time: Some(get_test_start_time()),
            tlcfi_log_file: RELATIVE_TLCFI_FILE_PATH.to_string(),
            vlog_tlcfi_mapping_file: RELATIVE_VLOG_MAPPING_FILE_PATH.to_string(),
        };

        run_with_args(app_args);
        let actual_vlog_output = read_to_string("./3031_20211215_110000.vlg").unwrap();

        let mut expected_lines = Vec::new();
        expected_lines.extend(expected_vlog_output.split_whitespace().into_iter());
        for (i, actual_line) in actual_vlog_output.split_whitespace().enumerate() {
            assert_eq!(actual_line, expected_lines[i]);
        }
    }
}
