use crate::modes::GameMode;
use crate::protos::gamelog::{
    chat_event::ChatType,
    time_event::ModeState,
    ChatEvent, GameEvent, GameLog, JoinEvent, LeaveEvent, Player, Team, TimeEvent,
};
use protobuf::MessageField;

pub fn build_demo_log(mode: GameMode) -> GameLog {
    let (map, winner) = match mode {
        GameMode::CAI => ("Prison Break (Demo)", "Cops"),
        GameMode::TIMV => ("Mansion (Demo)", "Innocents"),
        GameMode::BP => ("Battle Arena (Demo)", "Top 3"),
        GameMode::GRAV => ("Sky Islands (Demo)", "Finishers"),
        GameMode::BED => ("Bed Wars (Demo)", "Red"),
        GameMode::Halloween2023 | GameMode::Halloween2024 | GameMode::Halloween2025 => {
            ("Haunted House (Demo)", "Survivors")
        }
        GameMode::Turf2026 => ("Turf Fields (Demo)", "Blue"),
    };

    let start_ms = 1_725_000_000_000i64; // Stable timestamp for deterministic snapshots
    let end_ms = start_ms + 12 * 60 * 1000;

    let mut log = GameLog::new();
    log.set_game_start(start_ms);
    log.set_game_end(end_ms);
    log.set_start_players(8);
    log.set_map(map.to_string());
    log.set_winner(winner.to_string());

    let red_players = [
        ("Dream", Some("dreamwastaken")),
        ("Technoblade", None),
        ("Notch", None),
        ("Sapnap", None),
    ];
    let blue_players = [
        ("TommyInnit", None),
        ("WilburSoot", None),
        ("GeorgeNotFound", Some("gnf")),
        ("CaptainSparklez", None),
    ];

    let mut team_red = Team::new();
    team_red.set_name("Red".to_string());
    team_red.set_score(42);
    team_red.set_color('c' as i32);
    team_red.players = red_players
        .iter()
        .map(|&(name, nick)| demo_player(name, nick))
        .collect();

    let mut team_blue = Team::new();
    team_blue.set_name("Blue".to_string());
    team_blue.set_score(37);
    team_blue.set_color('9' as i32);
    team_blue.players = blue_players
        .iter()
        .map(|&(name, nick)| demo_player(name, nick))
        .collect();

    log.teams.push(team_red);
    log.teams.push(team_blue);

    log.events = vec![
        join_evt(0, "Dream", 0),
        join_evt(0, "TommyInnit", 1),
        join_evt(0, "Technoblade", 0),
        join_evt(0, "CaptainSparklez", 1),
        chat_evt(15_000, "Dream", "glhf", ChatType::GLOBAL),
        chat_evt(45_000, "TommyInnit", "this UI is kinda clean ngl", ChatType::GLOBAL),
        chat_evt(75_000, "GeorgeNotFound", "team push mid", ChatType::TEAM),
        leave_evt(6 * 60 * 1000, "Notch"),
        chat_evt(10 * 60 * 1000, "Technoblade", "gg", ChatType::GLOBAL),
    ];

    log
}

fn join_evt(time_ms: i32, player: &str, team: i32) -> TimeEvent {
    let mut join = JoinEvent::new();
    join.set_player(player.to_string());
    join.set_team(team);

    let mut evt = GameEvent::new();
    set_ext_message(&mut evt, 101, &join);
    time_event(time_ms, evt)
}

fn leave_evt(time_ms: i32, player: &str) -> TimeEvent {
    let mut leave = LeaveEvent::new();
    leave.set_player(player.to_string());

    let mut evt = GameEvent::new();
    set_ext_message(&mut evt, 102, &leave);
    time_event(time_ms, evt)
}

fn chat_evt(time_ms: i32, sender: &str, message: &str, chat_type: ChatType) -> TimeEvent {
    let mut chat = ChatEvent::new();
    chat.set_sender(sender.to_string());
    chat.set_message(message.to_string());
    chat.set_type(chat_type);
    if matches!(chat_type, ChatType::TEAM) {
        chat.set_team(1);
    }

    let mut evt = GameEvent::new();
    set_ext_message(&mut evt, 100, &chat);
    time_event(time_ms, evt)
}

fn time_event(time_ms: i32, evt: GameEvent) -> TimeEvent {
    let mut time_event = TimeEvent::new();
    time_event.event = MessageField::some(evt);
    time_event.set_time(time_ms);
    time_event.set_state(ModeState::GAME);
    time_event
}

fn demo_player(name: &str, nick: Option<&str>) -> Player {
    let mut player = Player::new();
    player.set_name(name.to_string());
    player.set_uuid(demo_uuid_bytes(name));
    if let Some(nick) = nick {
        player.set_nick(nick.to_string());
    }
    player
}

fn demo_uuid_bytes(seed: &str) -> Vec<u8> {
    let mut h1 = 0xcbf29ce484222325u64;
    let mut h2 = 0x84222325cbf29ce4u64;
    for b in seed.as_bytes() {
        h1 ^= u64::from(*b);
        h1 = h1.wrapping_mul(0x00000100000001B3);
        h2 ^= u64::from(*b);
        h2 = h2.wrapping_mul(0x00000100000001B3).rotate_left(7);
    }
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&h1.to_be_bytes());
    out.extend_from_slice(&h2.to_be_bytes());
    out
}

fn set_ext_message<M: protobuf::Message>(evt: &mut GameEvent, field_number: u32, msg: &M) {
    let bytes = msg.write_to_bytes().unwrap_or_default();
    evt.special_fields
        .mut_unknown_fields()
        .add_length_delimited(field_number, bytes);
}
