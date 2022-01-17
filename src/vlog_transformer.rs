//! Transform [TimestampedChanges](struct.TimestampedChanges.html) into a VLog3 messages.
pub mod vlog_transformer {

    use std::{
        collections::HashMap,
        fs::File,
        io::{BufRead, BufReader},
    };

    use chrono::{Datelike, Duration, NaiveDateTime, Timelike};

    use tlcfi_assimilator::TimestampedChanges;

    const TIME_REFERENCE_INTERVAL_IN_S: i64 = 300;

    // TODO Create enum for message types
    // TODO handle unwraps
    // TODO get rid of some to_string calls in favor of &str
    // TODO implement status messages every 5 minutes with the time reference messages. first handle all changes, and save the first change state of any entity, then build initial status message on that and insert in front

    /// Transforms the given Vec of [TimestampedChanges](struct.TimestampedChanges.html) into a Vec of Strings representing VLog3 messages.
    /// Other than the direct transformation of [TimestampedChanges](struct.TimestampedChanges.html) to change messages, an initial VLog info message is inserted in front.
    /// Time reference messages are also inserted every 5 minutes.
    ///
    /// Only the following types of VLog change messages are supported:
    /// * 6  - Detectie informatie
    /// * 14 - Externe signaalgroep status
    pub fn to_vlog(
        timestamped_changes_vec: Vec<TimestampedChanges>,
        start_date_time: NaiveDateTime,
        vlog_tlcfi_mapping_file: String,
    ) -> Vec<String> {
        let tlc_name = load_tlc_name(&vlog_tlcfi_mapping_file).unwrap();
        println!("Found tlc name: {}", tlc_name);
        let vlog_signal_name_mapping = load_mappings(&vlog_tlcfi_mapping_file, "Signals").unwrap();
        println!("Found signal mapping: {:#?}", &vlog_signal_name_mapping);
        let vlog_detector_name_mapping =
            load_mappings(&vlog_tlcfi_mapping_file, "Detectors").unwrap();
        println!("Found detector mapping: {:#?}", &vlog_detector_name_mapping);

        let mut vlog_messages: Vec<String> = Vec::new();

        let mut ms_of_last_time_reference = 0;

        vlog_messages.extend(insert_vlog_statuses(start_date_time, tlc_name));

        for timestamped_changes in timestamped_changes_vec {
            if timestamped_changes.ms_from_beginning - ms_of_last_time_reference
                > TIME_REFERENCE_INTERVAL_IN_S * 1000
            {
                vlog_messages.push(get_time_reference(
                    start_date_time,
                    timestamped_changes.ms_from_beginning,
                ));
                ms_of_last_time_reference = timestamped_changes.ms_from_beginning;
            }
            if !timestamped_changes.signal_names.is_empty() {
                vlog_messages.push(transform_signal_changes(
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

    fn load_tlc_name(file_name: &str) -> Result<String, String> {
        let mapping_file = File::open(file_name).unwrap();

        let reader = BufReader::new(mapping_file);
        let mut tlc_name = Option::None;
        let mut next_line_has_info = false;
        for line in reader.lines() {
            let read_line = line.unwrap().trim().to_string();
            if !next_line_has_info && read_line.contains("//") && read_line.contains("TLC") {
                next_line_has_info = true;
            }
            if next_line_has_info {
                if !read_line.contains("//") && !read_line.is_empty() {
                    tlc_name = Some(read_line.to_string());
                    break;
                }
            }
        }

        match tlc_name {
            Some(name) => Ok(name),
            None => Err("We didn't find a tlc name!".to_string()),
        }
    }

    fn load_mappings(file_name: &str, mapping_type: &str) -> Result<HashMap<String, i16>, String> {
        let mapping_file = File::open(file_name).unwrap();

        let reader = BufReader::new(mapping_file);
        let mut mappings = HashMap::new();

        let mut next_line_has_info = false;
        for line in reader.lines() {
            let read_line = line.unwrap().trim().to_string();

            if !next_line_has_info && read_line.contains("//") && read_line.contains(mapping_type) {
                next_line_has_info = true;
            } else if next_line_has_info && !read_line.is_empty() && !read_line.contains("//") {
                let mapping: Vec<&str> = read_line.split(",").collect();
                mappings.insert(
                    mapping[1].trim().to_string(),
                    mapping[0].trim().parse::<i16>().unwrap(),
                );
            } else if next_line_has_info {
                // "Stopping file parsings since we found an empty line when we expected info."
                break;
            }
        }

        if mappings.is_empty() {
            Err(format!(
                "No {} mappings found in the mapping file!",
                mapping_type
            ))
        } else {
            Ok(mappings)
        }
    }

    fn transform_signal_changes(
        signal_changes: TimestampedChanges,
        vlog_signal_name_mapping: &HashMap<String, i16>,
        ms_of_last_time_reference: i64,
    ) -> String {
        // The structure for a CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS
        // description  hex digits
        // type         2
        // time delta   3
        // data amount  1
        // amount times
        //   id         2
        //   state      2
        let message_type = "0E";

        let data_amount = format!("{:?}", signal_changes.signal_names.len());
        let static_string = format!(
            "{:}{:03X}{:}",
            message_type,
            from_tlcfi_time_to_vlog_time(
                signal_changes.ms_from_beginning - ms_of_last_time_reference
            ),
            data_amount
        );

        let mut dynamic_string = String::new();
        let mut vlog_ids_in_message: Vec<(usize, &i16)> = signal_changes
            .signal_names
            .iter()
            .enumerate()
            .map(|(index, name)| (index, vlog_signal_name_mapping.get(name as &str).unwrap()))
            .collect();
        vlog_ids_in_message.sort_by(|(_, id1), (_, id2)| id1.partial_cmp(id2).unwrap());
        for (index, signal_id) in vlog_ids_in_message {
            dynamic_string.push_str(&format!(
                "{:02X}{:02X}",
                signal_id,
                signal_changes.signal_states[index].to_vlog_state()
            ));
        }
        format!("{}{}", static_string, dynamic_string)
    }

    fn transform_detector_changes(
        detector_changes: TimestampedChanges,
        vlog_detector_name_mapping: &HashMap<String, i16>,
        ms_of_last_time_reference: i64,
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
            .map(|(index, name)| (index, vlog_detector_name_mapping.get(name as &str).unwrap()))
            .collect();
        vlog_ids_in_message.sort_by(|(_, id1), (_, id2)| id1.partial_cmp(id2).unwrap());
        for (index, vlog_id) in vlog_ids_in_message {
            dynamic_string.push_str(&format!(
                "{:02X}{:02X}",
                vlog_id,
                detector_changes.detector_states[index].to_vlog_state()
            ));
        }
        format!("{}{}", static_string, dynamic_string)
    }

    fn from_tlcfi_time_to_vlog_time(tlcfi_time: i64) -> i64 {
        tlcfi_time / 100
    }

    fn insert_vlog_statuses(start_date_time: NaiveDateTime, tlc_name: String) -> Vec<String> {   
        vec![get_time_reference(start_date_time, 0), get_vlog_info(tlc_name)]
    }

    fn get_time_reference(start_date_time: NaiveDateTime, ms_since_beginning: i64) -> String {
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
            .checked_add_signed(Duration::milliseconds(ms_since_beginning))
            .unwrap();
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

    fn get_vlog_info(tlc_name: String) -> String {
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
        let vlog_info = format!(
            "{:02X}{}{}",
            4, "030000", &encoded_tlc_name
        );
        vlog_info
    }
}
