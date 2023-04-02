use bevy::{prelude::Resource, render::extract_resource::ExtractResource};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ZoneTimeState {
    Morning,
    Day,
    Evening,
    Night,
}

#[derive(Clone, Resource, ExtractResource)]
pub struct ZoneTime {
    pub state: ZoneTimeState,
    pub state_percent_complete: f32,
    pub time: u32,
    pub debug_overwrite_time: Option<u32>,
}

impl Default for ZoneTime {
    fn default() -> Self {
        Self {
            state: ZoneTimeState::Morning,
            state_percent_complete: 0.0,
            time: 0,
            debug_overwrite_time: None,
        }
    }
}
