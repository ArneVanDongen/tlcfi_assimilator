//! Transform [TimestampedChanges](struct.TimestampedChanges.html) into a VLog3 messages.

use std::{
    collections::HashMap,
    convert::TryInto,
    fs::File,
    io::{BufRead, BufReader},
};

use chrono::{Datelike, Duration, NaiveDateTime, Timelike};

use tlcfi_assimilator::TimestampedChanges;

const TIME_REFERENCE_INTERVAL_IN_S: u64 = 300;

// TODO Create enum for message types
// TODO get rid of some to_string calls in favor of &str
// TODO implement status messages every 5 minutes with the time reference messages. first handle all changes, and save the first change state of any entity, then build initial status message on that and insert in front
// TODO merge common functionality of transform_signal_changes and transform_detector_changes

/// Transforms the given Vec of [TimestampedChanges](struct.TimestampedChanges.html) into a Vec of Strings representing VLog3 messages.
/// Other than the direct transformation of [TimestampedChanges](struct.TimestampedChanges.html) to change messages, an initial VLog info message is inserted in front.
/// Time reference messages are also inserted every 5 minutes.
///
/// Only the following types of VLog change messages are supported:
/// * 6  - Detectie informatie
/// * 14 - Externe signaalgroep status
pub fn to_vlog(
    timestamped_changes_vec: Vec<TimestampedChanges>,
    start_date_time: &NaiveDateTime,
    vlog_tlcfi_mapping_file: &str,
    tlc_name: &str,
) -> Vec<String> {
    let vlog_signal_name_mapping =
        load_mappings(&vlog_tlcfi_mapping_file, "Signals").expect(&format!(
            "Couldn't find Signal mappings in the given VLog TLC FI mapping file: {:?}",
            &vlog_tlcfi_mapping_file
        ));
    let vlog_detector_name_mapping =
        load_mappings(&vlog_tlcfi_mapping_file, "Detectors").expect(&format!(
            "Couldn't find Detector mappings in the given VLog TLC FI mapping file: {:?}",
            &vlog_tlcfi_mapping_file
        ));

    let mut vlog_messages: Vec<String> = Vec::new();

    let mut ms_of_last_time_reference = 0;

    vlog_messages.extend(insert_vlog_statuses(start_date_time, tlc_name));

    for timestamped_changes in timestamped_changes_vec {
        if timestamped_changes.ms_from_beginning - ms_of_last_time_reference
            >= TIME_REFERENCE_INTERVAL_IN_S * 1000
        {
            vlog_messages.push(get_time_reference(
                start_date_time,
                timestamped_changes.ms_from_beginning,
            ));
            ms_of_last_time_reference = timestamped_changes.ms_from_beginning;
        }
        if !timestamped_changes.signal_names.is_empty() {
            vlog_messages.extend(transform_signal_changes(
                timestamped_changes,
                &vlog_signal_name_mapping,
                ms_of_last_time_reference,
            ));
        } else if !timestamped_changes.detector_names.is_empty() {
            vlog_messages.push(transform_detector_changes(
                timestamped_changes,
                &vlog_detector_name_mapping,
                ms_of_last_time_reference,
            ));
        }
    }

    vlog_messages
}

pub fn load_tlc_name(file_name: &str) -> Option<String> {
    let mapping_file = File::open(file_name).expect(&format!(
        "Failed to open VLog TLC FI mapping file: {:?}",
        &file_name
    ));

    let reader = BufReader::new(mapping_file);
    let mut tlc_name = Option::None;
    let mut next_line_has_info = false;
    for line_res in reader.lines() {
        if let Ok(line) = line_res {
            let read_line = line.trim().to_string();
            if !next_line_has_info && read_line.contains("//") && read_line.contains("TLC") {
                next_line_has_info = true;
            }
            if next_line_has_info {
                if !read_line.contains("//") && !read_line.is_empty() {
                    tlc_name = Some(read_line.to_string());
                    break;
                }
            }
        } else {
            eprintln!("Failed to read line {:?}", line_res)
        }
    }

    tlc_name
}

fn load_mappings(
    file_name: &str,
    mapping_type: &str,
) -> Result<HashMap<String, i16>, Box<dyn std::error::Error>> {
    let mapping_file = File::open(file_name)?;

    let reader = BufReader::new(mapping_file);
    let mut mappings = HashMap::new();

    let mut next_line_has_info = false;
    for line in reader.lines() {
        let read_line = line?.trim().to_string();

        if !next_line_has_info && read_line.contains("//") && read_line.contains(mapping_type) {
            next_line_has_info = true;
        } else if next_line_has_info && !read_line.is_empty() && !read_line.contains("//") {
            let mapping: Vec<&str> = read_line.split(",").collect();
            mappings.insert(
                mapping[1].trim().to_string(),
                mapping[0].trim().parse::<i16>()?,
            );
        } else if next_line_has_info {
            // "Stopping file parsings since we found an empty line when we expected info."
            break;
        }
    }

    if mappings.is_empty() {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No {} mappings found in the mapping file!", mapping_type),
        )))
    } else {
        Ok(mappings)
    }
}

fn transform_signal_changes(
    signal_changes: TimestampedChanges,
    vlog_signal_name_mapping: &HashMap<String, i16>,
    ms_of_last_time_reference: u64,
) -> Vec<String> {
    // The structure for a CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS
    // description  hex digits
    // type         2
    // time delta   3
    // data amount  1
    // amount times
    //   id         2
    //   state      2
    let message_type = "0E";

    let data_limit_split_changes = split_changes_on_data_limit_signal(signal_changes, 4);

    let mut messages = Vec::new();

    for changes in data_limit_split_changes {
        let data_amount = format!("{:X}", changes.signal_names.len());
        let static_string = format!(
            "{:}{:03X}{:}",
            message_type,
            from_tlcfi_time_to_vlog_time(changes.ms_from_beginning - ms_of_last_time_reference),
            data_amount
        );

        let mut dynamic_string = String::new();
        let mut vlog_ids_in_message: Vec<(usize, &i16)> = changes
            .signal_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                (
                    index,
                    vlog_signal_name_mapping.get(name as &str).expect(&format!(
                        "Couldn't find TLC FI signal name '{:?}' in VLog mapping file",
                        name
                    )),
                )
            })
            .collect();
        vlog_ids_in_message.sort_by(|(_, id1), (_, id2)| {
            id1.partial_cmp(id2)
                .expect("Failed to compare VLog ids. This should never happen.")
        });
        for (index, signal_id) in vlog_ids_in_message {
            dynamic_string.push_str(&format!(
                "{:02X}{:02X}",
                signal_id,
                changes.signal_states[index].to_vlog_state()
            ));
        }
        messages.push(format!("{}{}", static_string, dynamic_string))
    }
    messages
}

fn split_changes_on_data_limit_signal(
    changes: TimestampedChanges,
    data_size: i32,
) -> Vec<TimestampedChanges> {
    let mut split_changes = Vec::new();
    let max_amount = 40 / data_size;
    let mut amount_in_timestampted_changes = 0;

    let mut last_timestamped_changes = TimestampedChanges {
        ms_from_beginning: changes.ms_from_beginning,
        ..Default::default()
    };

    for (i, name) in changes.signal_names.iter().enumerate() {
        last_timestamped_changes.signal_names.push(name.to_string());
        last_timestamped_changes.signal_states.push(changes.signal_states[i]);
        amount_in_timestampted_changes += 1;

        if amount_in_timestampted_changes == max_amount {
            split_changes.push(last_timestamped_changes);
            last_timestamped_changes = TimestampedChanges {
                ms_from_beginning: changes.ms_from_beginning,
                ..Default::default()
            };
            amount_in_timestampted_changes = 0;
        }
    }

    split_changes.push(last_timestamped_changes);

    split_changes
}

fn transform_detector_changes(
    detector_changes: TimestampedChanges,
    vlog_detector_name_mapping: &HashMap<String, i16>,
    ms_of_last_time_reference: u64,
) -> String {
    // The structure for a CHANGE_DETECTION_INFORMATION
    // description  hex digits
    // type         2
    // time delta   3
    // data amount  1
    // amount times
    //   id         2
    //   state      2
    let message_type = "06";

    let data_amount = format!("{:?}", detector_changes.detector_names.len());
    let static_string = format!(
        "{:}{:03X}{:}",
        message_type,
        from_tlcfi_time_to_vlog_time(
            detector_changes.ms_from_beginning - ms_of_last_time_reference
        ),
        data_amount
    );
    let mut dynamic_string = String::new();
    let mut vlog_ids_in_message: Vec<(usize, &i16)> = detector_changes
        .detector_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                index,
                vlog_detector_name_mapping
                    .get(name as &str)
                    .expect(&format!(
                        "Couldn't find TLC FI detector name '{:?}' in VLog mapping file",
                        name
                    )),
            )
        })
        .collect();
    vlog_ids_in_message.sort_by(|(_, id1), (_, id2)| {
        id1.partial_cmp(id2)
            .expect("Failed to compare VLog ids. This should never happen.")
    });
    for (index, vlog_id) in vlog_ids_in_message {
        dynamic_string.push_str(&format!(
            "{:02X}{:02X}",
            vlog_id,
            detector_changes.detector_states[index].to_vlog_state()
        ));
    }
    format!("{}{}", static_string, dynamic_string)
}

/// tlcfi time is in milliseconds, vlog time is in deciseconds
fn from_tlcfi_time_to_vlog_time(tlcfi_time: u64) -> u64 {
    tlcfi_time / 100
}

fn insert_vlog_statuses(start_date_time: &NaiveDateTime, tlc_name: &str) -> Vec<String> {
    vec![
        get_time_reference(start_date_time, 0),
        get_vlog_info(tlc_name),
    ]
}

fn get_time_reference(start_date_time: &NaiveDateTime, ms_since_beginning: u64) -> String {
    // #Tijd referentiebericht zie 2.1.
    // 012021043008002450
    // Elements of the time are encoded in a way that they are readable
    // Element  bit index   meaning
    // Year     63 - 48     the year 2021 is shown in hex as the value 0x2021
    // Month    47 - 40     etc
    // Day      39 - 32
    // Hour     31 - 24
    // Minute   23 - 16
    // Second   15 -  8
    // Tenths   7  -  4
    // empty    3  -  0
    let reference_time = start_date_time
        .checked_add_signed(Duration::milliseconds(
            ms_since_beginning
                .try_into()
                .expect("Failed to convert u64 into i64"),
        ))
        .expect(&format!(
            "Adding {:?} to date time {:?} caused an overflow error. Is our input correct?",
            ms_since_beginning, start_date_time
        ));
    let date_string = format!(
        "{:02}{:02}{:02}",
        reference_time.year(),
        reference_time.month(),
        reference_time.day()
    );
    let time_string = format!(
        "{:02}{:02}{:02}{:01}",
        reference_time.hour(),
        reference_time.minute(),
        reference_time.second(),
        reference_time.nanosecond() / 1_000_000_00
    );
    let time_reference = format!("{:02X}{}{}0", 1, date_string, time_string);
    time_reference
}

fn get_vlog_info(tlc_name: &str) -> String {
    // # V-Log informatie, zie 2.3
    // Has the following format <type><versie><vri_id>
    // message type is 4
    // version is 030000
    // 54494E5431 = TINT1
    // 44454D4F = DEMO
    let mut encoded_tlc_name = String::new();
    for (i, something) in tlc_name.encode_utf16().enumerate() {
        encoded_tlc_name.push_str(&format!("{:02X}", something));
        if i > 19 {
            break;
        }
    }
    if tlc_name.len() < 20 {
        for _ in tlc_name.len()..20 {
            encoded_tlc_name.push_str("20");
        }
    }
    let vlog_info = format!("{:02X}{}{}", 4, "030000", &encoded_tlc_name);
    vlog_info
}

mod test {

    use super::*;

    const TEST_TLC_NAME: &str = "test";

    fn get_test_start_date_time() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2021-12-15T11:00:00.000", "%FT%T%.3f")
            .expect("Use a valid time stamp for tests!")
    }

    fn get_test_vlog_signal_name_mapping() -> HashMap<String, i16> {
        [
            ("01".to_string(), 0),
            ("02".to_string(), 1),
            ("03".to_string(), 2),
            ("04".to_string(), 3),
            ("05".to_string(), 4),
            ("06".to_string(), 5),
            ("07".to_string(), 6),
            ("08".to_string(), 7),
            ("09".to_string(), 8),
            ("10".to_string(), 9),
            ("11".to_string(), 10),
            ("12".to_string(), 11),
            ("13".to_string(), 12),
            ("14".to_string(), 13),
            ("15".to_string(), 14),
            ("16".to_string(), 15),
            ("17".to_string(), 16),
            ("18".to_string(), 17),
            ("71".to_string(), 18),
        ]
        .iter()
        .cloned()
        .collect()
    }

    fn get_test_vlog_detector_name_mapping() -> HashMap<String, i16> {
        [("D712".to_string(), 4), ("D713".to_string(), 2)]
            .iter()
            .cloned()
            .collect()
    }

    #[test]
    fn get_vlog_info_should_transform_tlc_name_into_vlog_hex_message() {
        let expected_vlog_info = "040300007465737420202020202020202020202020202020";
        let actual_vlog_info = get_vlog_info(TEST_TLC_NAME);
        assert_eq!(actual_vlog_info, expected_vlog_info);
    }

    #[test]
    fn get_time_reference_should_create_a_time_reference_message_based_on_the_ms_since_beginning() {
        let expected_time_reference = "012021121511000520";
        let actual_time_reference = get_time_reference(&get_test_start_date_time(), 5212);
        assert_eq!(actual_time_reference, expected_time_reference);
    }

    #[test]
    fn from_tlcfi_time_to_vlog_time_should_transform_ms_into_ds() {
        let ms = 3400;
        let ds = 34;
        assert_eq!(from_tlcfi_time_to_vlog_time(ms), ds);
    }

    #[test]
    fn transform_signal_changes_should_create_a_vlog_signal_change_message() {
        let expected_signal_change_message = vec!["0E00320A021200"];
        let detector_changes = TimestampedChanges {
            ms_from_beginning: 530,
            signal_names: vec!["11".to_string(), "71".to_string()],
            signal_states: vec![
                tlcfi_assimilator::SignalState::Amber,
                tlcfi_assimilator::SignalState::Red,
            ],
            ..Default::default()
        };

        let actual_signal_change_message =
            transform_signal_changes(detector_changes, &get_test_vlog_signal_name_mapping(), 180);

        assert_eq!(actual_signal_change_message, expected_signal_change_message);
    }

    #[test]
    fn transforming_more_than_40_bits_of_changes_should_make_multiple_messages() {
        // 0E 0A1 A  0002 0102 0202 0302 0402 0502 0602 0702 0802 0902
        // 0E 255 10 0005 0105 0205 0305 0405 0505 0605 0705 0805 0905
        // 0E 255 18 0005 0105 0205 0305 0405 0505 0605 0705 0805 0905 0A05 0B05 0C05 0D05 0E05 0F05 1005 1105
        // 0E 003 18 0000 0100 0200 0300 0400 0500 0600 0700 0800 0900 0A00 0B00 0C00 0D00 0E00 0F00 1000 1100
        let expected_messages = vec!["0E003A0000010002000300040005000600070008000900", "0E00380A000B000C000D000E000F0010001100"];
        let detector_changes = TimestampedChanges {
            ms_from_beginning: 530,
            signal_names: vec![
                "01".to_string(),
                "02".to_string(),
                "03".to_string(),
                "04".to_string(),
                "05".to_string(),
                "06".to_string(),
                "07".to_string(),
                "08".to_string(),
                "09".to_string(),
                "10".to_string(),
                "11".to_string(),
                "12".to_string(),
                "13".to_string(),
                "14".to_string(),
                "15".to_string(),
                "16".to_string(),
                "17".to_string(),
                "18".to_string(),
            ],
            signal_states: vec![
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
                tlcfi_assimilator::SignalState::Red,
            ],
            ..Default::default()
        };

        let actual_signal_change_message =
            transform_signal_changes(detector_changes, &get_test_vlog_signal_name_mapping(), 180);

        assert_eq!(actual_signal_change_message, expected_messages);
    }

    #[test]
    fn transform_detector_changes_should_create_a_vlog_sensor_change_message() {
        let expected_sensor_change_message = "06005202000401";
        let detector_changes = TimestampedChanges {
            ms_from_beginning: 640,
            detector_names: vec!["D712".to_string(), "D713".to_string()],
            detector_states: vec![
                tlcfi_assimilator::DetectorState::OCCUPIED,
                tlcfi_assimilator::DetectorState::FREE,
            ],
            ..Default::default()
        };

        let actual_sensor_change_message = transform_detector_changes(
            detector_changes,
            &get_test_vlog_detector_name_mapping(),
            80,
        );

        assert_eq!(actual_sensor_change_message, expected_sensor_change_message);
    }
}
