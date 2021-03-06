//! # tlcfi_assimilator
//!
//! `tlcfi_assimilator` is a program that makes sense of tlcfi data generated by SmartTraffic logs.

use chrono::{NaiveDateTime};

/// A set of changes with a time delta to the first decoded message in milliseconds.
/// It will have either signal names and states, or detector names and states.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TimestampedChanges {
    pub ms_from_beginning: u64,
    pub signal_names: Vec<String>,
    pub signal_states: Vec<SignalState>,
    pub detector_names: Vec<String>,
    pub detector_states: Vec<DetectorState>,
}

#[derive(Debug)]
pub struct AssimilationData {
    pub start_time: NaiveDateTime,
    pub sorted_lines: Vec<String>,
    pub first_tick: Option<u64>,
    pub previous_tick: Option<u64>,
    pub bonus_ms: Option<u64>,
    pub changes: Vec<TimestampedChanges>,
}

impl Default for AssimilationData {
    fn default() -> Self { 
        Self {
            start_time: NaiveDateTime::parse_from_str("2015-09-05 23:56:04", "%Y-%m-%d %H:%M:%S").unwrap(),
            sorted_lines: Vec::new(),
            first_tick: Option::None,
            previous_tick: Option::None,
            bonus_ms: Option::None, 
            changes: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SignalState {
    Unavailable,
    Dark,
    Red,
    Amber,
    Green,
    AmberFlashing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DetectorState {
    FREE,
    OCCUPIED,
}

impl From<u64> for SignalState {
    /// Returns the [SignalState](enum.SignalState.html) corresponding to the given TLC-FI signal state represented as a number
    ///
    /// # Arguments
    ///
    /// * `tlc_fi_state` - A u64 that represents a TLC-FI signal state
    fn from(tlc_fi_state: u64) -> Self {
        match tlc_fi_state {
            0 => SignalState::Unavailable,
            1 => SignalState::Dark,
            2 | 3 => SignalState::Red,
            5 | 6 => SignalState::Green,
            7 | 8 => SignalState::Amber,
            9 => SignalState::AmberFlashing,
            _ => panic!(
                "Don't know what SignalState to transform '{}' into.",
                tlc_fi_state
            ),
        }
    }
}

impl SignalState {
    /// Transforms a [SignalState](enum.SignalState.html) to the value corresponding to that state in VLog
    pub fn to_vlog_state(&self) -> i16 {
        match self {
            Self::Unavailable => 4,
            Self::Dark => 4,
            Self::Red => 0,
            Self::Green => 1,
            Self::Amber => 2,
            Self::AmberFlashing => 5,
        }
    }
}

impl From<u64> for DetectorState {
    /// Returns the [DetectorState](enum.DetectorState.html) corresponding to the given TLC-FI detector state represented as a number
    ///
    /// # Arguments
    ///
    /// * `tlc_fi_state` - A u64 that represents a TLC-FI signal state
    fn from(tlc_fi_state: u64) -> Self {
        match tlc_fi_state {
            0 => DetectorState::FREE,
            1 => DetectorState::OCCUPIED,
            _ => panic!(
                "Don't know what DetectorState to transform '{}' into.",
                tlc_fi_state
            ),
        }
    }
}

impl DetectorState {
    /// Transforms a [DetectorState](enum.DetectorState.html) to the value corresponding to that state in VLog
    pub fn to_vlog_state(&self) -> i16 {
        match self {
            Self::FREE => 0,
            Self::OCCUPIED => 1,
        }
    }
}
