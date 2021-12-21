#![warn(missing_docs)]
pub mod vlog_transformer {

    use chrono::{Datelike, Duration, NaiveDate, Timelike};
    use std::collections::HashMap;

    use tlcfi_assimilator::TimestampedChanges;

    const TIME_REFERENCE_INTERVAL_IN_S: i64 = 300;

    // types
    // 5  - STATUS_DETECTION_INFORMATION - detector status
    // 6  - CHANGE_DETECTION_INFORMATION - detector change
    // 13 - STATUS_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal status
    // 14 - CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal change
    pub fn to_vlog(timestamped_changes_vec: Vec<TimestampedChanges>) -> Vec<String> {
        let vlog_signal_name_mapping: HashMap<&str, i16> = [
            ("02", 0),
            ("03", 1),
            ("04", 2),
            ("06", 3),
            ("07", 4),
            ("08", 5),
            ("58", 6),
            ("59", 7),
            ("61", 8),
            ("62", 9),
            ("68", 10),
            ("69", 11),
            ("71", 12),
        ]
        .iter()
        .cloned()
        .collect();
        let vlog_detector_name_mapping: HashMap<&str, i16> = [
            ("D611", 0),
            ("D612", 1),
            ("D621", 2),
            ("D622", 3),
            ("D623", 4),
            ("D624", 5),
            ("D625", 6),
            ("D626", 7),
            ("D681", 8),
            ("D682", 9),
            ("D683", 10),
            ("D684", 11),
            ("D685", 12),
            ("D691", 13),
            ("D692", 14),
            ("D693", 15),
            ("D581", 16),
            ("D582", 17),
            ("D627", 18),
            ("D711", 19),
            ("D712", 20),
            ("D713", 21),
            ("D714", 22),
            ("D029", 23),
            ("D039", 24),
            ("D628", 25),
            ("D629", 26),
            ("Drk481", 27),
            ("Drk491", 28),
        ]
        .iter()
        .cloned()
        .collect();

        let mut vlog_messages: Vec<String> = Vec::new();

        let mut ms_of_last_time_reference = 0;

        // TODO impl first messages in vlog cycle:
        // first handle all changes, and save the first change state of any entity, then build initial status message on that and insert in front
        vlog_messages.extend(insert_vlog_statuses());

        for timestamped_changes in timestamped_changes_vec {
            if timestamped_changes.ms_from_beginning - ms_of_last_time_reference
                > TIME_REFERENCE_INTERVAL_IN_S * 1000
            {
                vlog_messages.push(get_time_reference(timestamped_changes.ms_from_beginning));
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

    fn transform_signal_changes(
        signal_changes: TimestampedChanges,
        vlog_signal_name_mapping: &HashMap<&str, i16>,
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
        vlog_detector_name_mapping: &HashMap<&str, i16>,
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
        // TODO sort on vlog id
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

    fn insert_vlog_statuses() -> Vec<String> {
        // #Tijd referentiebericht zie 2.1.
        // 012021043008002450
        // TODO can pass start date as argument
        // 012021121511000000
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
        let time_reference = get_time_reference(0);

        // # V-Log informatie, zie 2.3
        // Has the following format <type><versie><vri_id>
        // version is 030000
        // 54494E5431 = TINT1
        // 44454D4F = DEMO
        let tlc_name = "3031";
        let mut encoded_tlc_name = String::new();
        for (i, something) in "3031".encode_utf16().enumerate() {
            encoded_tlc_name.push_str(&format!("{:02X}", something));
            if i > 19 {
                break;
            }
        }
        // Ehh fix this
        if tlc_name.len() < 20 {
            for _ in tlc_name.len() + 1..20 {
                encoded_tlc_name.push_str("20");
            }
        }
        println!("{:?}", &encoded_tlc_name);
        // "3330333120202020202020202020202020202020"
        let vlog_info = format!(
            "{:02X}{}{}",
            4, "030000", "3330333120202020202020202020202020202020"
        );

        // # Externe signaalgroep status [0..254] (WUS), zie 3.2.
        // 0D00000700000000
        // # Detectie informatie [0..254], zie 3.4.
        // 050000090000000000
        // Nice to have: cuteviewer inserts these
        vec![time_reference, vlog_info]
    }

    fn get_time_reference(ms_since_beginning: i64) -> String {
        let start_time = NaiveDate::from_ymd(2021, 12, 15).and_hms(11, 00, 00);
        let reference_time = start_time
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
}
