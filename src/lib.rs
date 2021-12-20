#[derive(Debug, Default)]
pub struct TimestampedChanges {
    pub ms_from_beginning: u64,
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
    pub fn to_vlog_state(&self) -> i16 {
        match self {
            Self::FREE => 0,
            Self::OCCUPIED => 1,
        }
    }
}
