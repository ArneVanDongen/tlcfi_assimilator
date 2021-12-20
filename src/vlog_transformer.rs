#![warn(missing_docs)]
pub mod vlog_transformer {

    use std::collections::HashMap;

    use tlcfi_assimilator::TimestampedChanges;

    // types
    // 5  - STATUS_DETECTION_INFORMATION - detector status
    // 6  - CHANGE_DETECTION_INFORMATION - detector change
    // 13 - STATUS_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal status
    // 14 - CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal change
    pub fn to_vlog(timestamped_changes_vec: Vec<TimestampedChanges>) -> String {
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

        // TODO impl first messages in vlog cycle: 
        // first handle all changes, and save the first change state of any entity, then build initial status message on that and insert in front

        let mut last_ms_since = 0;

        insert_vlog_statuses();

        for signal_changes in timestamped_changes_vec {
            last_ms_since = signal_changes.ms_from_beginning;
            if !signal_changes.signal_names.is_empty() {
                transform_signal_changes(signal_changes, &vlog_signal_name_mapping);
            } else if !signal_changes.detector_names.is_empty() {
                transform_detector_changes(
                    signal_changes,
                    &vlog_detector_name_mapping,
                );
            }
        }

        String::new()
    }

    fn transform_signal_changes(
        signal_changes: TimestampedChanges,
        vlog_signal_name_mapping: &HashMap<&str, i16>,
    ) {
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
        let static_string = format!("{:}{:03X}{:}", message_type, from_tlcfi_time_to_vlog_time(signal_changes.ms_from_beginning), data_amount);
        let mut dynamic_string = String::new();
        for (i, signal_name) in signal_changes.signal_names.iter().enumerate() {
            dynamic_string.push_str(&format!(
                "{:02X}{:02X}",
                vlog_signal_name_mapping.get(signal_name as &str).unwrap(),
                signal_changes.signal_states[i].to_vlog_state()
            ));
        }
        let full_string = format!("{}{}", static_string, dynamic_string);
        println!("{}", full_string);
    }


    fn transform_detector_changes(
        detector_changes: TimestampedChanges,
        vlog_detector_name_mapping: &HashMap<&str, i16>,
    ) {
        let message_type = "06";

        println!("Not yet implemented detector changes");
    }

    fn from_tlcfi_time_to_vlog_time(tlcfi_time: u64) -> u64 {
        tlcfi_time / 100
    }

    fn insert_vlog_statuses() {
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
        let time_reference = format!("{:02X}{}000", 1, "2021121511000");
        println!("{}", time_reference);
        // # V-Log informatie, zie 2.3
        // Has the following format <type><versie><vri_id> 
        // version is 030000
        // 54494E5431 = TINT1
        // 44454D4F = DEMO
        let vlog_info = format!("{:02X}{}{}", 4, "030000", "3330333120202020202020202020202020202020");
        println!("{}", vlog_info);

        // # Externe signaalgroep status [0..254] (WUS), zie 3.2. 
        // 0D00000700000000
        // # Detectie informatie [0..254], zie 3.4. 
        // 050000090000000000
        // Nice to have: cuteviewer inserts these
        
    }
}
