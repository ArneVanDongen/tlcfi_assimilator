use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

use json::JsonValue;

mod vlog_transformer;

use tlcfi_assimilator::TimestampedChanges;

// TODO handle all unwraps properly

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
    let filename = "tlcfi.txt";
    // Open the file in read-only mode (ignoring errors).
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    let mut first_tick = Option::None;

    let mut signal_changes: Vec<TimestampedChanges> = Vec::new();

    // Read the file line by line using the lines() iterator from std::io::BufRead.
    let mut reverse_lines = Vec::new();
    for line in reader.lines() {
        reverse_lines.insert(0, line.unwrap());
    }

    for line in reverse_lines {
        // println!("Looking at line: {:?}", line);
        let filtered_line = line.replace("\"\"", "\"");
        let split_line: Vec<&str> = filtered_line.split("- ").collect();

        // Only consider message from the TLC.
        if split_line[1].contains("IN") {
            if first_tick == Option::None {
                first_tick = find_first_tick(&split_line[2]);
            }
            if first_tick != Option::None {
                let stuff = parse_string(split_line[2], first_tick.unwrap());
                signal_changes.extend(stuff.unwrap());
            }
        }
    }

    let vlog_messages = vlog_transformer::vlog_transformer::to_vlog(signal_changes);

    let mut file = File::create("test.vlg").unwrap();
    for msg in vlog_messages {
        write!(file, "{}\r\n", msg).unwrap();
    }
}

fn find_first_tick(first_line_json: &str) -> Option<u64> {
    let json_res = json::parse(first_line_json);
    match json_res {
        Ok(json_obj) => match &json_obj["params"]["ticks"] {
            JsonValue::Number(number) => Option::Some(number.as_fixed_point_u64(0).unwrap()),
            _ => Option::None,
        },
        Err(_) => Option::None,
    }
}

fn parse_string(json_str: &str, first_tick: u64) -> Result<Vec<TimestampedChanges>, &str> {
    let json_res = json::parse(json_str);
    match json_res {
        Ok(json_obj) => Ok(parse_json(json_obj, first_tick)),
        Err(_) => Err("Failed to parse json string"),
    }
}

fn parse_json(json_obj: JsonValue, first_tick: u64) -> Vec<TimestampedChanges> {
    let timestamped_changes = vec![];

    let message_type = match &json_obj["params"]["update"][0]["objects"]["type"] {
        JsonValue::Number(number) => number.as_fixed_point_u64(0).unwrap(),
        _ => 0,
    };

    if message_type != 3 {
        // TODO read detector messages
        parse_change_json(
            json_obj,
            first_tick,
            timestamped_changes,
            ChangeType::Detector,
        )
    } else {
        parse_change_json(
            json_obj,
            first_tick,
            timestamped_changes,
            ChangeType::Signal,
        )
    }
}

fn parse_change_json(
    json_obj: JsonValue,
    first_tick: u64,
    mut timestamped_changes: Vec<TimestampedChanges>,
    change_type: ChangeType,
) -> Vec<TimestampedChanges> {
    let ms_from_beginning = find_ms_from_beginning(&json_obj, first_tick) as i64;

    let update = &json_obj["params"]["update"][0];

    if update["objects"]["ids"] == JsonValue::Null {
        timestamped_changes
    } else {
        let ids_vec = match &update["objects"]["ids"] {
            JsonValue::Array(vec) => vec,
            _ => panic!("yeet"),
        };

        let states_vec = match &update["states"] {
            JsonValue::Array(vec) => vec,
            _ => panic!("yeet"),
        };

        assert_eq!(ids_vec.len(), states_vec.len(), "We assume that the amount of IDs and detector states is equal, but for the following tlc-fi json it wasn't:\n{:#}.", json_obj);

        let mut names = Vec::new();
        let mut states: Vec<u64> = Vec::new();
        for (i, id) in ids_vec.iter().enumerate() {
            let name: &str = match id {
                JsonValue::Short(short) => short.as_str(),
                JsonValue::String(string) => string,
                _ => panic!("yeet"),
            };

            let state_num = match &states_vec[i]["state"] {
                JsonValue::Number(number) => number.as_fixed_point_u64(0).unwrap(),
                JsonValue::Null => continue,
                _ => panic!("yeet"),
            };

            &names.push(name.to_string());
            &states.push(state_num.into());
        }

        if !names.is_empty() {
            match change_type {
                ChangeType::Detector => {
                    let mut detector_states = Vec::new();
                    for state in states {
                        detector_states.push(state.into());
                    }
                    timestamped_changes.push(TimestampedChanges {
                        ms_from_beginning,
                        detector_names: names,
                        detector_states,
                        ..Default::default()
                    });
                }
                ChangeType::Signal => {
                    let mut signal_states = Vec::new();
                    for state in states {
                        signal_states.push(state.into());
                    }
                    timestamped_changes.push(TimestampedChanges {
                        ms_from_beginning,
                        signal_names: names,
                        signal_states,
                        ..Default::default()
                    });
                }
            }
        }
        timestamped_changes
    }
}

fn find_ms_from_beginning(json_obj: &JsonValue, first_tick: u64) -> u64 {
    match json_obj["params"]["ticks"] {
        JsonValue::Number(number) => {
            let tick = number.as_fixed_point_u64(0).unwrap();
            if tick < first_tick {
                eprintln!(
                    "Tick in message ({:?}) wasn't bigger than initial tick!",
                    &tick
                );
                0
            } else {
                number.as_fixed_point_u64(0).unwrap() - first_tick
            }
        }
        _ => {
            //TODO handle this better
            1
        }
    }
}

enum ChangeType {
    Detector,
    Signal,
}
