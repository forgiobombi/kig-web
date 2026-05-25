// Copyright (C) 2021 RoccoDev
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
    db::GameLogMeta,
    error::Result,
    modes::GameMode,
    protos::gamelog::{self, chat_event::ChatType, GameLog, TimeEvent},
    web::get_current_time,
    AppState,
};
use actix_web::{
    http::header::ContentType,
    web, HttpResponse,
};
use askama::Template;
use cached::{proc_macro::cached, TimedCache};
use event::EventType::{self, *};
use gamelog::{BukkitDamageCause, ChatEvent, GameEvent};
use regex::Regex;
use serde::Serialize;
use std::{borrow::Cow, str::FromStr};
use std::{collections::HashMap, convert::TryInto, fmt, time::Duration};
use time::UtcDateTime;

mod bed;
mod bp;
mod cai;
mod demo;
mod event;
mod grav;
mod halloween;
mod timv;
mod turf;

lazy_static::lazy_static! {
    static ref MAP_ESCAPE_REGEX: Regex = Regex::new(r#"[^a-zA-Z0-9]"#).unwrap();
    static ref SPECTATORS: Team<'static> = Team {
      name: "Spec",
      players: vec![],
      score: 0,
      color: mc_to_rgb('7')
    };
}
static ANON_UUID: &str = "c06f8906-4c8a-4911-9c29-ea1dbd1aab82";
static DEFAULT_COLORS: [char; 2] = ['c', 'e'];

#[derive(Template)]
#[template(path = "gamelog.html")]
struct GamelogTemplate<'a> {
    log: &'a GameLog,
    total_players: usize,
    game_id: &'a str,
    teams: Vec<Team<'a>>,
    events: Vec<WrappedEvent>,
    player_teams: PlayerTeamMap<'a>,
    winner: Option<Team<'a>>,
    mode: GameMode,
    functions: Functions,
    extension: WrappedExtension,
    server: Option<String>,
    current_year: String,
    nicks_hidden: bool,
    player_display_names: HashMap<String, String>,
    player_uuids: HashMap<String, String>,
    player_nicks: HashMap<String, String>,
    player_stats_json: String,
}

// Extensions - each mode can implement its own version
pub struct Functions {
    extension: Box<dyn GameLogExtension>,
}

pub trait GameLogExtension {
    fn parse_event(&self, event: &GameEvent) -> event::EventType;
    fn get_box_color(&self, event: &EventType) -> &'static str;
    fn supports_score(&self) -> bool {
        true
    }
    fn get_map<'slf, 'log: 'slf>(&'slf self, log: &'log GameLog) -> Cow<'slf, str> {
        Cow::Borrowed(log.map())
    }
}

#[derive(Clone)]
pub enum WrappedExtension {
    Cai(cai::CaiExtension),
    Timv(timv::TimvExtension),
    Bp(bp::BpExtension),
    Grav(grav::GravExtension),
    Bed(bed::BedExtension),
    Halloween(halloween::HalloweenExtension),
    Turf(turf::TurfExtension),
}

impl WrappedExtension {
    fn boxed(self) -> Box<dyn GameLogExtension> {
        use self::WrappedExtension::*;
        match self {
            Cai(ext) => Box::new(ext),
            Timv(ext) => Box::new(ext),
            Bp(ext) => Box::new(ext),
            Grav(ext) => Box::new(ext),
            Bed(ext) => Box::new(ext),
            Halloween(ext) => Box::new(ext),
            Turf(ext) => Box::new(ext),
        }
    }
}

impl GameMode {
    fn to_gamelog_ext(&self, log: &GameLog) -> WrappedExtension {
        use self::WrappedExtension::*;
        match self {
            GameMode::CAI => Cai(cai::CaiExtension {}),
            GameMode::TIMV => Timv(timv::TimvExtension {}),
            GameMode::BP => Bp(bp::BpExtension {}),
            GameMode::GRAV => Grav(grav::GravExtension::new(log)),
            GameMode::BED => Bed(bed::BedExtension::new(log)),
            GameMode::Halloween2023 | GameMode::Halloween2024 | GameMode::Halloween2025 => {
                Halloween(halloween::HalloweenExtension {})
            }
            GameMode::Turf2026 => Turf(turf::TurfExtension {}),
        }
    }
}

/// Represents a Java UUID
#[derive(Clone, Copy)]
pub struct UUID {
    lsb: u64,
    msb: u64,
}

#[derive(Clone, Copy)]
pub struct Player<'a> {
    pub uuid: UUID,
    pub name: &'a str,
    pub nick: Option<&'a str>,
}

#[derive(Clone)]
pub struct Team<'a> {
    pub name: &'a str,
    pub players: Vec<Player<'a>>,
    pub score: i32,
    pub color: &'static str,
}

pub struct WrappedEvent {
    id: usize,
    event: EventType,
    time: i32,
}

enum ChatChannel<'a> {
    Static(&'static str),
    Team(&'a str, &'static str),
    None,
}

pub struct PlayerTeamMap<'a>(HashMap<&'a str, Vec<(usize, &'a Team<'a>)>>);

#[derive(Default, Serialize)]
struct PlayerStats {
    kills: u32,
    deaths: u32,
    joins: u32,
    leaves: u32,
    messages: u32,
    extra: HashMap<String, u32>,
}

#[cached(
    ty = "TimedCache<(Vec<u8>, GameMode), (GameLog, GameLogMeta)>",
    create = "{ TimedCache::with_lifespan(Duration::from_secs(120)) }",
    convert = "{ (id.clone(), mode) }",
    result
)]
async fn get_log(
    state: web::Data<AppState>,
    mode: GameMode,
    id: Vec<u8>,
) -> Result<(GameLog, GameLogMeta)> {
    state
        .db
        .game_log_by_id(mode.get_database_id(), id.clone())
        .await
        .and_then(|opt| opt.ok_or(crate::error::Error::NotFound))
}

pub async fn gamelog_by_id(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (mut mode, path_id) = path.into_inner();

    match base62::decode(&path_id) {
        Ok(id) => {
            mode.make_ascii_uppercase();

            let mode = GameMode::from_str(&mode)
                .map_err(|_| crate::error::Error::ModeNotFound)?;

            let (log, meta) =
                get_log(state, mode, id.to_be_bytes()[2..].to_vec()).await?;

            render_gamelog(mode, &path_id, &log, meta.server)
        }
        Err(_) => Ok(HttpResponse::BadRequest().body("Invalid game ID")),
    }
}

pub async fn demo_gamelog(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let mut mode = path.into_inner();

    mode.make_ascii_uppercase();

    let mode = GameMode::from_str(&mode)
        .map_err(|_| crate::error::Error::ModeNotFound)?;

    let log = demo::build_demo_log(mode);

    render_gamelog(
        mode,
        "DEMO",
        &log,
        Some(String::from("Demo server")),
    )
}

fn render_gamelog(
    mode: GameMode,
    game_id: &str,
    log: &GameLog,
    server: Option<String>,
) -> Result<HttpResponse> {
    let teams: Vec<Team> = log
        .teams
        .iter()
        .enumerate()
        .map(|(i, t)| Team {
            name: t.name(),
            score: t.score(),
            color: get_team_color(t, i),
            players: t
                .players
                .iter()
                .map(|p| Player {
                    name: p.name(),
                    uuid: p.uuid().into(),
                    nick: p.has_nick().then(|| p.nick()),
                })
                .collect(),
        })
        .collect();
    let winner = log.has_winner().then(|| log.winner()).and_then(|winner| {
        teams
            .iter()
            .find(|t| t.name == winner)
            .cloned()
            .or_else(|| {
                Some(Team {
                    name: winner,
                    color: "",
                    score: 0,
                    players: vec![],
                })
            })
    });

    let extension = mode.to_gamelog_ext(log);
    let extension_ptr = extension.clone().boxed();

    let events: Vec<WrappedEvent> = log
        .events
        .iter()
        .enumerate()
        .map(|(i, e)| WrappedEvent::parse(i, e, &*extension_ptr))
        .collect();

    let player_teams = PlayerTeamMap::new(&teams, &events);
    let player_display_names = teams
        .iter()
        .flat_map(|t| t.players.iter())
        .map(|p| (p.name.to_string(), p.nick.unwrap_or(p.name).to_string()))
        .collect();
    let player_uuids = teams
        .iter()
        .flat_map(|t| t.players.iter())
        .map(|p| (p.name.to_string(), p.uuid.to_string()))
        .collect();
    let player_nicks = teams
        .iter()
        .flat_map(|t| t.players.iter())
        .filter_map(|p| p.nick.map(|nick| (p.name.to_string(), nick.to_string())))
        .collect();

    let current_time = get_current_time();
    let nicks_hidden = if log.has_nick_embargo() && log.nick_embargo() > 0 && log.has_game_start()
    {
        UtcDateTime::from_unix_timestamp(log.game_start() / 1000).is_ok_and(|game_time| {
            game_time + time::Duration::seconds(log.nick_embargo().into()) > current_time
        })
    } else {
        false
    };

    let player_stats_json = build_player_stats_json(&events, nicks_hidden, &player_nicks);

    let render = GamelogTemplate {
        log,
        total_players: if log.start_players() == 0 {
            log.teams.iter().map(|t| t.players.len()).sum()
        } else {
            log.start_players() as usize
        },
        game_id,
        teams: teams.clone(),
        events,
        player_teams,
        winner,
        mode,
        functions: Functions {
            extension: extension_ptr,
        },
        extension,
        server,
        current_year: current_time.year().to_string(),
        nicks_hidden,
        player_display_names,
        player_uuids,
        player_nicks,
        player_stats_json,
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(render))
}

fn build_player_stats_json(
    events: &[WrappedEvent],
    nicks_hidden: bool,
    player_nicks: &HashMap<String, String>,
) -> String {
    fn stats_key(player: &str, nicks_hidden: bool, player_nicks: &HashMap<String, String>) -> String {
        if nicks_hidden {
            player_nicks
                .get(player)
                .cloned()
                .unwrap_or_else(|| player.to_string())
        } else {
            player.to_string()
        }
    }

    fn bump(extra: &mut HashMap<String, u32>, key: &str) {
        *extra.entry(key.to_string()).or_insert(0) += 1;
    }

    let mut stats: HashMap<String, PlayerStats> = HashMap::new();

    for wrapped in events {
        match &wrapped.event {
            EventType::Chat(chat) => {
                let key = stats_key(chat.sender(), nicks_hidden, player_nicks);
                stats.entry(key).or_default().messages += 1;
            }
            EventType::Join(join) => {
                let key = stats_key(join.player(), nicks_hidden, player_nicks);
                stats.entry(key).or_default().joins += 1;
            }
            EventType::Leave(leave) => {
                let key = stats_key(leave.player(), nicks_hidden, player_nicks);
                stats.entry(key).or_default().leaves += 1;
            }
            EventType::CaiDeath(death) => {
                let victim_key = stats_key(death.player(), nicks_hidden, player_nicks);
                stats.entry(victim_key).or_default().deaths += 1;
                if death.has_killer() {
                    let killer_key = stats_key(death.killer(), nicks_hidden, player_nicks);
                    stats.entry(killer_key).or_default().kills += 1;
                }
            }
            EventType::CaiCatch(catch) => {
                let leader_key = stats_key(catch.leader(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(leader_key).or_default().extra, "Caught");
                let carrier_key = stats_key(catch.carrier(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(carrier_key).or_default().extra,
                    "Catches",
                );
            }
            EventType::CaiCapture(capture) => {
                let leader_key = stats_key(capture.leader(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(leader_key).or_default().extra,
                    "Captured",
                );
                let carrier_key = stats_key(capture.carrier(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(carrier_key).or_default().extra,
                    "Captures",
                );
            }
            EventType::CaiEscape(escape) => {
                let leader_key = stats_key(escape.leader(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(leader_key).or_default().extra,
                    "Escapes",
                );
                if escape.has_saver() {
                    let saver_key = stats_key(escape.saver(), nicks_hidden, player_nicks);
                    bump(&mut stats.entry(saver_key).or_default().extra, "Saves");
                }
            }
            EventType::TimvDeath(death) => {
                let victim_key = stats_key(death.player(), nicks_hidden, player_nicks);
                stats.entry(victim_key).or_default().deaths += 1;
                if death.has_killer() {
                    let killer_key = stats_key(death.killer(), nicks_hidden, player_nicks);
                    stats.entry(killer_key).or_default().kills += 1;
                }
            }
            EventType::TimvTest(test) => {
                let player_key = stats_key(test.player(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(player_key).or_default().extra, "Tests");
            }
            EventType::TimvTrap(trap) => {
                let player_key = stats_key(trap.player(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(player_key).or_default().extra, "Traps");
            }
            EventType::TimvBody(body) => {
                let identifier_key = stats_key(body.identifier(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(identifier_key).or_default().extra,
                    "Bodies Found",
                );
            }
            EventType::TimvDetectiveBody(det) => {
                let identifier_key = stats_key(det.identifier(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(identifier_key).or_default().extra,
                    "Inspections",
                );
            }
            EventType::TimvPsychicReport(report) => {
                let psychic_key = stats_key(report.psychic(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(psychic_key).or_default().extra,
                    "Reports",
                );
            }
            EventType::TimvSharedPurchase(purchase) => {
                let purchaser_key = stats_key(purchase.purchaser(), nicks_hidden, player_nicks);
                bump(
                    &mut stats.entry(purchaser_key).or_default().extra,
                    "Purchases",
                );
            }
            EventType::BpPowerup(powerup) => {
                let player_key = stats_key(powerup.name(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(player_key).or_default().extra, "Powerups");
            }
            EventType::BpDeath(death) => {
                for eliminated in &death.player {
                    let player_key = stats_key(eliminated.name(), nicks_hidden, player_nicks);
                    stats.entry(player_key).or_default().deaths += 1;
                }
            }
            EventType::GravStageCompletion(stage) => {
                let player_key = stats_key(stage.player(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(player_key).or_default().extra, "Stages");
            }
            EventType::GravGameFinish(finish) => {
                let player_key = stats_key(finish.player(), nicks_hidden, player_nicks);
                bump(&mut stats.entry(player_key).or_default().extra, "Finishes");
            }
            EventType::GravHardcoreFail(fail) => {
                let player_key = stats_key(fail.player(), nicks_hidden, player_nicks);
                stats.entry(player_key.clone()).or_default().deaths += 1;
                bump(
                    &mut stats.entry(player_key).or_default().extra,
                    "Hardcore Deaths",
                );
            }
            EventType::BedBedDestruction(bed) => {
                if bed.has_player() {
                    let player_key = stats_key(bed.player(), nicks_hidden, player_nicks);
                    bump(
                        &mut stats.entry(player_key).or_default().extra,
                        "Beds Broken",
                    );
                }
            }
            EventType::HerdDeath(death) => {
                let victim_key = stats_key(death.player(), nicks_hidden, player_nicks);
                stats.entry(victim_key.clone()).or_default().deaths += 1;
                if death.has_killer() {
                    let killer_key = stats_key(death.killer(), nicks_hidden, player_nicks);
                    stats.entry(killer_key).or_default().kills += 1;
                }
                if !death.respawn() {
                    bump(
                        &mut stats.entry(victim_key).or_default().extra,
                        "Final Deaths",
                    );
                }
            }
            EventType::HalloweenDeath(death) => {
                let victim_key = stats_key(death.player(), nicks_hidden, player_nicks);
                stats.entry(victim_key).or_default().deaths += 1;
                if death.has_killer() {
                    let killer_key = stats_key(death.killer(), nicks_hidden, player_nicks);
                    stats.entry(killer_key).or_default().kills += 1;
                }
            }
            EventType::TurfDeath(death) => {
                let victim_key = stats_key(death.player(), nicks_hidden, player_nicks);
                stats.entry(victim_key).or_default().deaths += 1;
                if death.has_killer() {
                    let killer_key = stats_key(death.killer(), nicks_hidden, player_nicks);
                    stats.entry(killer_key).or_default().kills += 1;
                }
            }
            _ => {}
        }
    }

    serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
}

impl Functions {
    fn get_box_color(&self, event: &WrappedEvent) -> &str {
        match event.get_raw_event() {
            EventType::Chat(_) => "",
            EventType::Join(_) => "list-group-item-info",
            EventType::Leave(_) => "list-group-item-dark",
            _ => self.extension.get_box_color(&event.event),
        }
    }

    fn get_map<'slf, 'log: 'slf>(&'slf self, log: &'log GameLog) -> Cow<'slf, str> {
        self.extension.get_map(log)
    }
}

impl WrappedEvent {
    fn parse(id: usize, event: &TimeEvent, extension: &dyn GameLogExtension) -> Self {
        let time = event.time();
        WrappedEvent {
            id,
            time,
            event: Self::parse_event(event, extension),
        }
    }

    /// Attempts to parse the event, interpreting it as a default event if possible.
    fn parse_event(event: &TimeEvent, extension: &dyn GameLogExtension) -> EventType {
        let event = &event.event;
        match extension.parse_event(event) {
            EventType::Unknown => {
                use crate::protos::gamelog::exts::*;
                if let Some(event) = chat.get(event) {
                    EventType::Chat(event)
                } else if let Some(event) = join.get(event) {
                    EventType::Join(event)
                } else if let Some(event) = leave.get(event) {
                    EventType::Leave(event)
                } else {
                    EventType::Unknown
                }
            }
            event => event,
        }
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn get_time(&self) -> i32 {
        self.time
    }

    fn get_raw_event(&self) -> &EventType {
        &self.event
    }

    fn get_chat_channel<'a>(
        &self,
        event_id: &usize,
        event: &ChatEvent,
        log: &GamelogTemplate<'a>,
    ) -> ChatChannel<'a> {
        if let EventType::Chat(chat_event) = &self.event {
            match chat_event.type_() {
                ChatType::LOBBY => ChatChannel::Static("Lobby"),
                ChatType::TEAM => if event.has_team() {
                    log.teams.get(event.team() as usize)
                } else {
                    log.player_teams.get_team_at(event.sender(), *event_id)
                }
                .map(|t| ChatChannel::Team(t.name, t.color))
                .unwrap_or_else(|| ChatChannel::Team(SPECTATORS.name, SPECTATORS.color)),
                ChatType::SHOUT => ChatChannel::Static("Shout"),
                ChatType::BROADCAST => ChatChannel::Static("Broadcast"),
                ChatType::GLOBAL => ChatChannel::None,
            }
        } else {
            ChatChannel::None
        }
    }

    fn is_chat(&self) -> bool {
        matches!(self.event, EventType::Chat(_))
    }
}

impl BukkitDamageCause {
    fn get_damage_desc(&self) -> &'static str {
        match self {
            BukkitDamageCause::ENTITY_ATTACK => "Melee",
            BukkitDamageCause::PROJECTILE => "Projectile",
            BukkitDamageCause::VOID => "Void",
            BukkitDamageCause::SUFFOCATION => "Suffocation",
            BukkitDamageCause::FIRE | BukkitDamageCause::FIRE_TICK => "Fire",
            BukkitDamageCause::FALL => "Fall",
            BukkitDamageCause::DROWNING => "Drowning",
            BukkitDamageCause::LAVA => "Lava",
            BukkitDamageCause::OTHER => "Unknown cause",
        }
    }
}

impl<'a> PlayerTeamMap<'a> {
    fn new(teams: &'a [Team<'a>], events: &[WrappedEvent]) -> Self {
        let mut res = Self(
            teams
                .iter()
                .flat_map(|t| t.players.iter().map(move |p| (p.name, vec![(0, t)])))
                .collect(),
        );
        // Team change events
        for event in events {
            if let EventType::Join(join) = &event.event {
                let player = join.player();
                let team = teams.get(join.team() as usize).unwrap_or(&SPECTATORS);
                if let Some(teams) = res.0.get_mut(player) {
                    teams.push((event.id, team));
                }
            }
        }
        res
    }

    fn get_team_at(&self, player: &str, event_id: usize) -> Option<&Team<'a>> {
        self.0
            .get(player)
            .and_then(|teams| teams.iter().rev().find(|(min_id, _)| event_id >= *min_id))
            .map(|(_, team)| team)
            .copied()
    }
}

impl From<&[u8]> for UUID {
    fn from(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 16);
        UUID {
            msb: u64::from_be_bytes(TryInto::try_into(&bytes[0..8]).unwrap()),
            lsb: u64::from_be_bytes(TryInto::try_into(&bytes[8..16]).unwrap()),
        }
    }
}

impl fmt::Display for UUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}-{}",
            &digits(self.msb >> 32, 8)[1..],
            &digits(self.msb >> 16, 4)[1..],
            &digits(self.msb, 4)[1..],
            &digits(self.lsb >> 48, 4)[1..],
            &digits(self.lsb, 12)[1..]
        )
    }
}

fn get_team_color(team: &gamelog::Team, idx: usize) -> &'static str {
    mc_to_rgb(if team.has_color() {
        team.color() as u8 as char
    } else {
        DEFAULT_COLORS[idx]
    })
}

/// Converts a Minecraft color to an RGB for CSS
#[inline]
fn mc_to_rgb(mc: char) -> &'static str {
    match mc {
        '1' => "#0000AA",
        '2' => "#019101",
        '3' => "#028e8e",
        '4' => "#AA0000",
        '5' => "#AA00AA",
        '6' => "#FFAA00",
        '7' => "#AAAAAA",
        '8' => "#555555",
        '9' => "#5555FF",
        'a' | 'A' => "#07d307",
        'b' | 'B' => "#0ccfcf",
        'c' | 'C' => "#e00b0b",
        'd' | 'D' => "#FF55FF",
        'e' | 'E' => "#cc901e",
        'f' | 'F' => "#FFFFFF",
        _ => "#000000",
    }
}

fn digits(val: u64, n: usize) -> String {
    let high = 1u64 << (n * 4usize);
    format!("{:x}", (high | val & (high - 1u64)))
}

fn format_duration(millis: i32) -> String {
    let minutes = millis / (1000 * 60);
    let seconds = millis / 1000 % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

mod filters {
    use crate::web::gamelog::{GamelogTemplate, PlayerTeamMap};

    pub use super::grav::filters::*;
    use std::borrow::Cow;

    use super::Team;

    pub fn format_duration(millis: &i64) -> askama::Result<String> {
        Ok(super::format_duration(*millis as i32))
    }

    pub fn format_duration_i32(millis: &i32) -> askama::Result<String> {
        Ok(super::format_duration(*millis))
    }

    pub fn map_file_name(map_name: &str) -> askama::Result<String> {
        if map_name.is_empty() {
            return Ok(String::from("default"));
        }
        let mut res: String = super::MAP_ESCAPE_REGEX.replace_all(map_name, "").into();
        res.make_ascii_lowercase();
        Ok(res)
    }

    pub fn player_name<'a>(player: &'a str, log: &'a GamelogTemplate) -> askama::Result<&'a str> {
        Ok(log
            .player_display_names
            .get(player)
            .map(|s| s.as_str())
            .unwrap_or(player))
    }

    pub fn player_uuid(player: &str, log: &GamelogTemplate) -> askama::Result<String> {
        if log.nicks_hidden && log.player_nicks.contains_key(player) {
            return Ok(super::ANON_UUID.to_string());
        }
        Ok(log
            .player_uuids
            .get(player)
            .cloned()
            .unwrap_or_else(|| super::ANON_UUID.to_string()))
    }

    pub fn player_key(player: &str, log: &GamelogTemplate) -> askama::Result<String> {
        if log.nicks_hidden {
            Ok(log
                .player_nicks
                .get(player)
                .cloned()
                .unwrap_or_else(|| player.to_string()))
        } else {
            Ok(player.to_string())
        }
    }

    pub fn team_from_idx<'a>(idx: &'a i32, teams: &'a [Team<'a>]) -> askama::Result<&'a Team<'a>> {
        Ok(teams.get(*idx as usize).unwrap_or(&super::SPECTATORS))
    }

    pub fn team_from_idx_u32<'a>(
        idx: &'a u32,
        teams: &'a [Team<'a>],
    ) -> askama::Result<&'a Team<'a>> {
        Ok(teams.get(*idx as usize).unwrap_or(&super::SPECTATORS))
    }

    pub fn team_color<'a>(
        player: &'a str,
        player_teams: &'a PlayerTeamMap<'a>,
        event_id: &usize,
    ) -> askama::Result<Cow<'a, str>> {
        Ok(
            match player_teams.get_team_at(player, *event_id).map(|t| t.color) {
                Some(color) => Cow::Owned(format!("{} !important", color)),
                None => Cow::Borrowed("#000000"),
            },
        )
    }
}
