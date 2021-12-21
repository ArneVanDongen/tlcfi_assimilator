//! # tlcfi_assimilator
//!
//! `tlcfi_assimilator` is a program that makes sense of tlcfi data generated by SmartTraffic logs.

/// A set of changes with a time delta to the first decoded message in milliseconds.
/// It will have either signal names and states, or detector names and states.
#[derive(Debug, Default)]
pub struct TimestampedChanges {
    pub ms_from_beginning: i64,
    pub signal_names: Vec<String>,
    pub signal_states: Vec<SignalState>,
    pub detector_names: Vec<String>,
    pub detector_states: Vec<DetectorState>,
}

#[derive(Debug)]
pub enum SignalState {
    UNAVAILABLE,
    DARK,
    RED,
    AMBER,
    GREEN,
}

#[derive(Debug)]
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
            0 => SignalState::UNAVAILABLE,
            1 => SignalState::DARK,
            2 | 3 => SignalState::RED,
            5 | 6 => SignalState::GREEN,
            7 | 8 => SignalState::AMBER,
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
            Self::UNAVAILABLE => 4,
            Self::DARK => 4,
            Self::RED => 0,
            Self::GREEN => 1,
            Self::AMBER => 2,
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
