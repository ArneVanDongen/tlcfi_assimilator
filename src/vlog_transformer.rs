#![warn(missing_docs)]
pub mod vlog_transformer {

    use std::{
        collections::HashMap,
    };

    use tlcfi_assimilator::TimestampedChanges;

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
        // types
        // 5  - STATUS_DETECTION_INFORMATION - detector status
        // 6  - CHANGE_DETECTION_INFORMATION - detector change
        // 13 - STATUS_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal status
        // 14 - CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS - signal change
    
        //TODO impl first messages in vlog cycle
    
        // The structure for a CHANGE_EXTERNAL_SIGNALGROUP_STATUS_WUS
        // description  hex digits
        // type         2
        // time delta   3
        // data amount  1
        // amount times
        //   id         2
        //   state      2
    
        let message_type = "0E";
        let mut last_ms_since = 0;
    
        for signal_changes in timestamped_changes_vec {
            let delta_time_ds = format!(
                "{:03X}",
                (signal_changes.ms_from_beginning - last_ms_since) / 100
            );
            last_ms_since = signal_changes.ms_from_beginning;
            let data_amount = format!("{:?}", signal_changes.signal_names.len());
            let static_string = format!("{:}{:}{:}", message_type, delta_time_ds, data_amount);
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
        String::new()
    }


}
