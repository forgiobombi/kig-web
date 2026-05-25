use std::borrow::Cow;

use crate::protos::gamelog::GameLog;

use super::{event::EventType, GameLogExtension};
use crate::protos::grav::exts::log_ext;

#[derive(Clone)]
pub struct GravExtension {
    stages: Vec<String>,
}

impl GravExtension {
    pub fn new(log: &GameLog) -> GravExtension {
        GravExtension {
            stages: log_ext
                .get(log)
                .map(|l| l.stages.iter().cloned().collect())
                .unwrap_or_else(|| Vec::new()),
        }
    }
}

impl GameLogExtension for GravExtension {
    fn get_box_color(&self, event: &super::EventType) -> &'static str {
        match event {
            EventType::GravGameFinish(_) => "list-group-item-success",
            EventType::GravStageCompletion(_) => "list-group-item-primary",
            EventType::GravHardcoreFail(_) => "list-group-item-danger",
            _ => "",
        }
    }

    fn parse_event(&self, event: &crate::protos::gamelog::GameEvent) -> EventType {
        use crate::protos::grav::exts::*;
        if let Some(event) = game_finish.get(event) {
            EventType::GravGameFinish(event)
        } else if let Some(event) = stage_completion.get(event) {
            EventType::GravStageCompletion(event)
        } else if let Some(event) = hardcore_fail.get(event) {
            EventType::GravHardcoreFail(event)
        } else {
            EventType::Unknown
        }
    }

    fn supports_score(&self) -> bool {
        false
    }

    fn get_map<'slf, 'log: 'slf>(&'slf self, _log: &'log GameLog) -> Cow<'slf, str> {
        Cow::Owned(self.stages.join(", "))
    }
}

pub(crate) mod filters {
    use std::borrow::Cow;

    use crate::web::gamelog::WrappedExtension;

    pub fn grav_format_time(nanos: &u64) -> askama::Result<String> {
        let millis = nanos / 1_000_000;

        let mins = millis / (1000 * 60);
        let secs = millis / 1000 % 60;
        let ms = millis % 1000;
        Ok(format!("{:#02}:{:#02}.{:#03}", mins, secs, ms))
    }

    pub fn grav_stage_name<'a>(
        index: &'a u32,
        ext: &'a WrappedExtension,
    ) -> askama::Result<Cow<'a, str>> {
        if let WrappedExtension::Grav(grav) = ext {
            Ok(Cow::Borrowed(
                grav.stages
                    .get(*index as usize)
                    .expect("invalid stage index"),
            ))
        } else {
            Ok(Cow::Owned(String::new()))
        }
    }
}
