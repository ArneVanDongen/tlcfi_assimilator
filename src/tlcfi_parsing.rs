// TODO handle all unwraps properly
// TODO refactor all unformative panics

use json::{parse, JsonValue};

use tlcfi_assimilator::TimestampedChanges;

pub fn find_first_tick(first_line_json: &str) -> Option<u64> {
    let json_res = parse(first_line_json);
    match json_res {
        Ok(json_obj) => match &json_obj["params"]["ticks"] {
            JsonValue::Number(number) => Option::Some(number.as_fixed_point_u64(0).unwrap()),
            _ => Option::None,
        },
        Err(_) => Option::None,
    }
}

pub fn parse_string(json_str: &str, first_tick: u64) -> Result<Vec<TimestampedChanges>, &str> {
    let json_res = parse(json_str);
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

    match message_type {
        3 => parse_change_json(
            json_obj,
            first_tick,
            timestamped_changes,
            ChangeType::Signal,
        ),
        4 => parse_change_json(
            json_obj,
            first_tick,
            timestamped_changes,
            ChangeType::Detector,
        ),
        _ => Vec::new(),
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

#[cfg(test)]
mod test {
    use super::*;
    use tlcfi_assimilator;

    const TEST_DETECTOR_JSON: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087808637,\"update\":[{\"objects\":{\"ids\":[\"D713\"],\"type\":4},\"states\":[{\"state\":1}]}]}}";

    const TEST_SIGNAL_JSON: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"UpdateState\",\"params\":{\"ticks\":4087808851,\"update\":[{\"objects\":{\"ids\":[\"71\"],\"type\":3},\"states\":[{\"state\":6}]}]}}";

    const FIRST_TICK: u64 = 4087807987;

    #[test]
    fn first_tick_should_be_tick_of_the_message() {
      assert_eq!(find_first_tick(TEST_DETECTOR_JSON), Some(4087808637));
    }

    #[test]
    fn detector_change_jsons_should_be_parsed_properly() -> Result<(), String> {
      let expected_changes = vec![tlcfi_assimilator::TimestampedChanges {
        ms_from_beginning: 650, 
        detector_names: vec!["D713".to_string()], 
        detector_states: vec![tlcfi_assimilator::DetectorState::OCCUPIED],
        ..Default::default()
      }];
      
      assert_eq!(parse_string(TEST_DETECTOR_JSON, FIRST_TICK)?, expected_changes);
      Ok(())
    }

    #[test]
    fn signal_change_jsons_should_be_parsed_properly() -> Result<(), String> {
      let expected_changes = vec![tlcfi_assimilator::TimestampedChanges {
        ms_from_beginning: 864, 
        signal_names: vec!["71".to_string()], 
        signal_states: vec![tlcfi_assimilator::SignalState::GREEN],
        ..Default::default()
      }];
      
      assert_eq!(parse_string(TEST_SIGNAL_JSON, FIRST_TICK)?, expected_changes);
      Ok(())
    }
}
