//! Bug Reports (ADR 0009): the core writes down every input it takes in and what came of
//! it, and saves it, with crops of what it read, as a `.zip` that replays: a new core fed
//! the same inputs goes through the same states. The core is driven here as the shell's
//! loop drives it, on the made-up game world.

mod common;

use common::items::{ItemWorld, equip, etc, item_world};
use common::osms;
use common::{FakeSource, SUNSET_SKY_ID, frame};
use minerva_core::bug_report::{self, Answer, Event, Note, Recording, ReplayData, MAX_REPORT_BYTES};
use minerva_core::{
    Action, BESTS_VERSION, Minerva, ExpBar, ExpBarReading, ExpBarRegion, Foreground, Frame, GameWindow, ItemCapture, ItemTooltip,
    ItemTooltipReading, MapBests, Rect, ReportAbout, SavedBests, ScreenPoint, ScreenRect, Settings, Size, TitleLines,
    TitleReading, TitleRegion, TrackingState, VisitBest,
};

const S: u64 = 1000;
const MIN: u64 = 60 * S;
const HOUR: u64 = 60 * MIN;

/// The wall clock at capture time 0: 11 Oct 2026, 12:00 UTC.
const WALL_AT_START: u64 = 1_791_720_000_000;

const CONSTRUCTION_SITE: (&str, &str) = ("Victoria Road", "Construction Site North of Kerning City");
const SUNSET_SKY: (&str, &str) = ("Victoria Road", "Sunset Sky");

fn about() -> ReportAbout {
    ReportAbout {
        version: "0.7.0".into(),
        os: "Windows 11, build 26100".into(),
        display_scale: 1.5,
        local_time: "2026-10-11 14:03:27".into(),
    }
}

/// The game window, at the screen's origin.
fn game() -> GameWindow {
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    GameWindow { client: screen, frame: screen, screens: vec![screen] }
}

/// The percentage the game shows for `exp` on a level of `total` EXP, to hundredths.
fn percent(exp: u64, total: u64) -> u32 {
    ((exp * 20_000 + total) / (2 * total)) as u32
}

/// The core as the shell runs it: the settings and Personal Bests restored at launch, then
/// a loop that every 50 ms reports the game window, what is in front and the Grind
/// Tracker's size, tells the wall clock and ticks, and takes the readings as they come.
struct Shell {
    minerva: Minerva,
    at: u64,
}

impl Shell {
    fn new(source: FakeSource) -> Self {
        let client = Size { width: 1920, height: 1200 };
        let mut minerva = Minerva::new(source);
        minerva.restore(Settings {
            title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
            exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
            ..Settings::default()
        });
        minerva.restore_bests(SavedBests::default());
        let mut shell = Shell { minerva, at: 0 };
        shell.step();
        shell
    }

    /// One turn of the loop, at the time it is now.
    fn step(&mut self) {
        let minerva = &mut self.minerva;
        minerva.on_game_window(Some(game()));
        minerva.on_foreground(Foreground::Game);
        minerva.set_grind_tracker_size(Size { width: 300, height: 200 });
        minerva.on_wall_clock(WALL_AT_START + self.at);
        minerva.tick(self.at);
    }

    /// Play on until `until`: the title on `map` read every 350 ms, and the EXP bar every
    /// second as `exp` says (`None`: not whole) at that time.
    fn play(&mut self, until: u64, map: (&str, &str), exp: impl Fn(u64) -> Option<ExpBar>) {
        while self.at < until {
            self.at += 50;
            if self.at % 350 == 0 {
                let lines = Some(TitleLines { street: map.0.into(), map: map.1.into() });
                self.minerva.on_reading(TitleReading { lines, at_ms: self.at });
            }
            if self.at % S == 0 {
                self.minerva.on_exp_bar_reading(ExpBarReading { bar: exp(self.at), at_ms: self.at });
            }
            self.step();
        }
    }

    fn report(&mut self) -> (minerva_core::BugReport, Recording) {
        let report = self.minerva.bug_report(&about());
        let recording = bug_report::read(&report.zip).expect("the report reads back");
        (report, recording)
    }
}

/// A grind: 18,000 EXP of a 20,000 EXP level at the start, 25 more a second, a death of
/// 400 EXP at 0:30, a misread at 0:40, and a level-up into a 25,000 EXP level at 1:37.
fn grind(at: u64) -> Option<ExpBar> {
    let s = at / S;
    let exp = 18_000 + 25 * s - if s >= 30 { 425 } else { 0 };
    if s == 40 {
        return Some(ExpBar { exp: 84_350, percent_hundredths: 4172 });
    }
    if exp >= 20_000 {
        let exp = exp - 20_000;
        return Some(ExpBar { exp, percent_hundredths: percent(exp, 25_000) });
    }
    Some(ExpBar { exp, percent_hundredths: percent(exp, 20_000) })
}

#[test]
fn a_report_replays_through_the_same_states_with_the_runs_osms_set() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);
    shell.play(3 * MIN, SUNSET_SKY, |at| grind(at).map(|bar| ExpBar { exp: bar.exp + 10, ..bar }));

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
}

#[test]
fn a_visit_removed_in_the_personal_bests_window_replays_and_reads_in_the_timeline() {
    // Sunset Sky's Visits at 10,800 and 7,200 EXP/h, the second set 11 Oct 2026, 11:00 UTC.
    let mut shell = Shell::new(FakeSource::osms());
    let removed = VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(WALL_AT_START - HOUR) };
    let visits = vec![VisitBest { exp_per_hour: 10_800, set_at_unix_ms: None }, removed];
    let maps = vec![MapBests { id: SUNSET_SKY_ID.into(), visits }];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, maps });
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    shell.minerva.remove_visit(SUNSET_SKY_ID, removed);
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let rates: Vec<u64> = shell.minerva.bests().maps[0].visits.iter().map(|v| v.exp_per_hour).collect();
    assert_eq!(rates, [10_800]);
    let (_, recording) = shell.report();
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
    let said = format!("Visit removed from {SUNSET_SKY_ID}'s Visit History: 7,200 EXP/h, set at {}", WALL_AT_START - HOUR);
    assert!(recording.timeline(0).iter().any(|line| line.ends_with(&said)), "{:#?}", recording.timeline(0));
}

#[test]
fn a_replay_with_another_osms_set_diverges_where_that_shows() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::made_up_world(3).files()));

    let divergence = replay.divergence.expect("another set takes the core elsewhere");
    let recorded = divergence.recorded.expect("a line the report has");
    assert!(recorded.contains("Construction Site North of Kerning City"), "where the map was recognised: {recorded}");
}

#[test]
fn a_replay_with_a_set_that_shows_the_same_goes_through_the_same_states() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    // A set fetched on another day, the same but for a map no one went to.
    let newer = osms::game().naming(osms::HENESYS_HUNTING_GROUND_I, "Henesys Hunting Ground One");
    let replay = bug_report::replay(&recording, Some(newer.files()));

    assert_eq!(replay.divergence, None);
}

#[test]
fn the_trace_says_what_the_session_made_of_the_readings() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let noted = |note: fn(&Note) -> bool| {
        recording.lines.iter().filter(|line| line.notes.iter().any(note)).map(|line| &line.event).collect::<Vec<_>>()
    };

    assert_eq!(noted(|n| *n == Note::SessionStarted).len(), 1);
    assert_eq!(noted(|n| *n == Note::Visit).len(), 1, "one map confirmed");
    let deaths = noted(|n| matches!(n, Note::Death { exp_lost: 400 }));
    assert!(matches!(deaths[..], [Event::ExpBar { bar: Some(ExpBar { exp: 18_325, .. }), .. }]), "{deaths:?}");
    let misreads = noted(|n| matches!(n, Note::Misfit { count: 1, .. }));
    assert!(matches!(misreads[..], [Event::ExpBar { bar: Some(ExpBar { exp: 84_350, .. }), .. }]), "{misreads:?}");
    let level_ups = noted(|n| matches!(n, Note::LevelUp { .. }));
    assert_eq!(level_ups.len(), 1, "{level_ups:?}");
}

#[test]
fn the_trace_says_what_the_grind_tracker_and_the_title_showed_where_it_changed() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let maps: Vec<_> = recording.lines.iter().filter_map(|line| line.map.as_ref()).collect();
    let states: Vec<_> =
        recording.lines.iter().filter_map(|line| line.grind_tracker.as_ref()?.get("state")?.as_str()).collect();

    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].id.as_deref(), Some("010003080"));
    assert_eq!(states, ["waiting", "warmingUp", "active"]);
    let last = shell.minerva.panel_state().exp_tracking.clone();
    assert_eq!(last.state, TrackingState::Active);
}

#[test]
fn ticks_that_change_nothing_are_left_out_but_for_the_last_before_a_line() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let ticks = recording.lines.iter().filter(|line| line.event == Event::Tick {}).count();
    let ticked = recording.lines.iter().filter(|line| line.ticked).count();

    // 1,200 turns of the loop, each with a tick.
    assert!(ticks < 60, "{ticks} ticks written");
    assert!(ticked > 200, "the readings come after a tick: {ticked}");
}

#[test]
fn inputs_that_only_set_a_value_are_written_when_it_changes() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    shell.minerva.on_foreground(Foreground::OtherApp);
    shell.minerva.on_foreground(Foreground::OtherApp);

    let (_, recording) = shell.report();
    let count = |is: fn(&Event) -> bool| recording.lines.iter().filter(|line| is(&line.event)).count();

    assert_eq!(count(|e| matches!(e, Event::GameWindow { .. })), 1);
    assert_eq!(count(|e| matches!(e, Event::Foreground { .. })), 2);
    assert_eq!(count(|e| matches!(e, Event::GrindTrackerSize { .. })), 1);
}

#[test]
fn the_wall_clock_is_written_down_again_only_when_it_strays() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    // The PC's clock set 10 s on.
    shell.minerva.on_wall_clock(WALL_AT_START + shell.at + 10 * S);
    shell.minerva.tick(shell.at + 50);

    let (_, recording) = shell.report();
    let wall_clocks: Vec<u64> = recording
        .lines
        .iter()
        .filter_map(|line| match line.event {
            Event::WallClock { unix_ms } => Some(unix_ms),
            _ => None,
        })
        .collect();

    assert_eq!(wall_clocks, [WALL_AT_START, WALL_AT_START + MIN + 10 * S]);
}

/// An EXP Bar Region frame captured at `at_ms`, its pixels telling `exp` apart.
fn exp_bar_frame(exp: u64, at_ms: u64) -> Frame {
    let shade = (exp % 251) as u8;
    frame(&image::RgbaImage::from_pixel(240, 32, image::Rgba([shade, shade, shade, 255])), at_ms)
}

/// Read the EXP bar from a frame every second from `from` to `until` as `exp` says.
fn read_frames(shell: &mut Shell, until: u64, exp: impl Fn(u64) -> ExpBar) {
    while shell.at < until {
        shell.at += S;
        let bar = exp(shell.at);
        let reading = ExpBarReading { bar: Some(bar), at_ms: shell.at };
        shell.minerva.on_exp_bar_frame_read(exp_bar_frame(bar.exp, shell.at), reading);
        shell.step();
    }
}

#[test]
fn the_exp_bars_crops_are_kept_where_they_changed_for_10_minutes_and_for_the_run_where_they_misread() {
    let mut shell = Shell::new(FakeSource::osms());
    // 1,000 EXP of a 20,000 EXP level, 5 more every 4 s, misread once at 0:30.
    let exp = |at: u64| {
        let exp = 1000 + 5 * (at / (4 * S));
        if at == 30 * S {
            return ExpBar { exp: 84_350, percent_hundredths: 4172 };
        }
        ExpBar { exp, percent_hundredths: percent(exp, 20_000) }
    };
    read_frames(&mut shell, 20 * MIN, exp);

    let (_, recording) = shell.report();
    let crop = |at: u64| format!("crops/exp-bar-{at:010}.png");

    assert!(recording.crops.contains(&crop(30 * S)), "the misread's crop stays for the run");
    assert!(!recording.crops.contains(&crop(8 * S)), "an ordinary crop goes after 10 minutes");
    assert!(recording.crops.contains(&crop(20 * MIN)), "a crop that changed, in the last 10 minutes");
    assert!(!recording.crops.contains(&crop(20 * MIN - S)), "one that didn't change isn't kept");
    // A crop every 4 s for 10 minutes, the misread, and the one after it.
    assert!((150..=153).contains(&recording.crops.len()), "{} crops", recording.crops.len());
}

/// A frame of noise, which no PNG compresses: about 470 KB.
fn noise_frame(seed: u64, at_ms: u64) -> Frame {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let rgba = (0..400 * 300 * 4)
        .map(|_| {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (state >> 56) as u8
        })
        .collect();
    Frame { width: 400, height: 300, rgba, at_ms }
}

#[test]
fn a_report_stays_under_5_mb_leaving_out_the_oldest_ordinary_crops_first() {
    let mut shell = Shell::new(FakeSource::osms());
    // The level learned, then 20 crops of noise a minute apart, the first a misread.
    let first = ExpBar { exp: 1000, percent_hundredths: 500 };
    shell.minerva.on_exp_bar_reading(ExpBarReading { bar: Some(first), at_ms: 30 * S });
    for n in 0..20 {
        shell.at += MIN;
        let bar = if n == 0 {
            ExpBar { exp: 84_350, percent_hundredths: 4172 }
        } else {
            ExpBar { exp: 1000 + n, percent_hundredths: percent(1000 + n, 20_000) }
        };
        shell.minerva.on_exp_bar_frame_read(noise_frame(n, shell.at), ExpBarReading { bar: Some(bar), at_ms: shell.at });
        shell.step();
    }

    let (report, recording) = shell.report();

    assert!(report.zip.len() <= MAX_REPORT_BYTES, "{} bytes", report.zip.len());
    assert!(recording.report.crops_left_out > 0);
    assert_eq!(recording.report.crops + recording.report.crops_left_out, 12, "the last 10 minutes and the kept one");
    assert!(recording.crops.contains(&format!("crops/exp-bar-{:010}.png", MIN)), "the kept one stays");
    assert!(recording.crops.contains(&format!("crops/exp-bar-{:010}.png", 20 * MIN)), "the newest stays");
}

#[test]
fn errors_from_the_shell_and_the_windows_are_in_the_trace_and_the_summary() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.note_error("shell", "capture failed: the screen is locked");
    shell.play(20 * S, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();
    let summary = text_in(&report.zip, "summary.txt");

    assert!(recording.lines.iter().any(|line| line.event
        == Event::Error { from: "shell".into(), message: "capture failed: the screen is locked".into() }));
    assert!(summary.contains("shell error: capture failed: the screen is locked"), "{summary}");
}

#[test]
fn the_summary_says_what_showed_at_the_press_and_what_the_core_made_of_things() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (report, _) = shell.report();
    let summary = text_in(&report.zip, "summary.txt");

    assert!(summary.contains("Version: 0.7.0"), "{summary}");
    assert!(summary.contains("Saved: 2026-10-11 14:03:27, 0:02:00 after Minerva started"), "{summary}");
    assert!(summary.contains("Map: Victoria Road · Construction Site North of Kerning City (010003080)"), "{summary}");
    assert!(summary.contains("Grind Tracker: Active"), "{summary}");
    assert!(summary.contains("0:00:30  EXP bar read 18,325 [91.63%]: death: 400 EXP lost"), "{summary}");
    assert!(summary.contains("level-up"), "{summary}");
}

#[test]
fn the_report_holds_the_maps_shown_with_their_map_summaries_and_no_osms_files() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    shell.play(2 * MIN, SUNSET_SKY, grind);

    let (report, recording) = shell.report();
    let ids: Vec<&str> = recording.report.maps.iter().map(|map| map.id.as_str()).collect();

    assert_eq!(ids, ["010003070", "010003080"]);
    assert!(recording.report.maps.iter().all(|map| map.summary.is_some()));
    assert_eq!(recording.report.osms_sets, [bug_report::fingerprint(&osms::game().files())]);
    let names = zip_names(&report.zip);
    assert!(names.iter().all(|name| !name.ends_with("maps.json") && !name.ends_with("monsters.json")), "{names:?}");
}

#[test]
fn the_report_is_named_by_version_and_local_time() {
    let mut shell = Shell::new(FakeSource::osms());

    let (report, recording) = shell.report();

    assert_eq!(report.file_name, "minerva-bug-report-0.7.0-2026-10-11-140327.zip");
    assert_eq!(recording.report.version, "0.7.0");
    assert_eq!(recording.report.os, "Windows 11, build 26100");
}

#[test]
fn a_run_over_a_day_keeps_the_last_day_and_the_lines_that_set_it_up() {
    let mut shell = Shell::new(FakeSource::osms());
    // A long line a minute for 30 hours: errors are the cheapest.
    for n in 0..30 * 60 {
        shell.at += MIN;
        shell.minerva.note_error("shell", &format!("{n:0>2000}"));
        shell.step();
    }

    let (_, recording) = shell.report();

    // The trace goes a chunk at a time, about 2 hours of these lines.
    assert!(recording.report.truncated);
    assert!((3 * HOUR..=6 * HOUR).contains(&recording.report.trace_from_ms), "{}", recording.report.trace_from_ms);
    assert!(matches!(recording.lines[0].event, Event::Restore { .. }), "{:?}", recording.lines[0]);
    assert!(matches!(recording.lines[1].event, Event::RestoreBests { .. }), "{:?}", recording.lines[1]);
    assert!(recording.lines[2].t >= recording.report.trace_from_ms);
}

#[test]
fn the_timeline_reads_what_came_in_and_what_came_of_it() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let timeline = recording.timeline(110 * S);

    assert!(timeline.iter().any(|line| line.starts_with("0:00:40  EXP bar read 84,350 [41.72%]  →  doesn't fit")), "{timeline:#?}");
    assert!(timeline.iter().any(|line| line.contains("map Victoria Road · Construction Site North of Kerning City (010003080)")));
    assert!(timeline.iter().any(|line| line.contains("level-up")));
    assert!(timeline.iter().any(|line| line.starts_with("0:01:55  EXP bar read")), "every reading from 1:50 on");
    assert!(!timeline.iter().any(|line| line.starts_with("0:01:05  EXP bar read")), "only what's notable before");
}

fn zip_names(zip: &[u8]) -> Vec<String> {
    let archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    archive.file_names().map(String::from).collect()
}

fn text_in(zip: &[u8], name: &str) -> String {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    let mut text = String::new();
    archive.by_name(name).expect(name).read_to_string(&mut text).expect("text");
    text
}

/// Made-up item data for the Item Card: two Blue Moons to pick between, and Bronze Ore.
fn items() -> ItemWorld {
    item_world(vec![
        etc(4_010_000, "Bronze Ore").sells_for(450),
        equip(1_041_078, "Blue Moon", "Top").sells_for(11_000),
        equip(1_032_011, "Blue Moon", "Earring").sells_for(10_500),
    ])
}

/// The Item Tooltip by the pointer read as `name`, after the player pressed Ctrl+Shift+I.
fn item_card(shell: &mut Shell, name: &str) {
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: 900, y: 620, width: 200, height: 19 }, name: Some(name.into()) };
    let pointer = ScreenPoint { x: 900, y: 600 };
    shell.minerva.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer, at_ms: shell.at }));
}

/// A run with Item Cards: the market prices kept restored (as 0.8.0 saved them), a card and
/// its market price typed (first not a price, then one), Market Lines added (first with a
/// name taken), renamed and removed, prices a tool of the player's own left (ADR 0013), a
/// choice and its pick, the card's MeowDB page and its ×.
fn run_with_item_cards() -> Shell {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    let kept = r#"{"version":1,"items":{"01041078":{"mesos":30000,"setAtUnixMs":1791700000000}}}"#;
    shell.minerva.restore_market_prices(serde_json::from_str(kept).unwrap());
    shell.minerva.set_item_card_size(Size { width: 280, height: 160 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    item_card(&mut shell, "Bronze Ore");
    shell.minerva.edit_card_market_price("1");
    shell.minerva.enter_card_market("lots");
    shell.minerva.enter_card_market("450k");
    shell.minerva.act(Action::AddCardMarketLine);
    shell.minerva.enter_card_market_line("your price");
    shell.minerva.enter_card_market_line("Lowest ask");
    shell.minerva.edit_card_market_price("2");
    shell.minerva.enter_card_market("400k");
    shell.minerva.edit_card_market_line("2");
    shell.minerva.enter_card_market_line("Cheapest");
    shell.minerva.edit_card_market_line("1");
    shell.minerva.act(Action::AskRemoveCardMarketLine);
    shell.minerva.act(Action::CancelCardMarket);
    shell.minerva.edit_card_market_line("2");
    shell.minerva.act(Action::AskRemoveCardMarketLine);
    shell.minerva.act(Action::RemoveCardMarketLine);
    let import = r#"{"version":2,"lines":[{"id":"tool-typical","name":"Your price","prices":{"04010000":{"mesos":500000,"setAtUnixMs":1791700000000}}}]}"#;
    shell.minerva.import_market_prices(serde_json::from_str(import).unwrap());
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    item_card(&mut shell, "Blue Moon");
    shell.minerva.pick_card_item("01032011");
    shell.minerva.open_card_meowdb_page(shell.at);
    shell.play(30 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.close_item_card();
    shell.play(40 * S, CONSTRUCTION_SITE, grind);
    shell
}

#[test]
fn item_cards_replay_through_the_same_states_with_the_runs_item_files() {
    let (_, recording) = run_with_item_cards().report();

    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    let replay = bug_report::replay_with(&recording, with);

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
}

#[test]
fn searches_replay_through_the_same_states_with_the_runs_files() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.minerva.set_item_card_size(Size { width: 280, height: 160 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_hotkey("ctrl+shift+f");
    shell.minerva.search_query("sunset");
    shell.minerva.search_pick_destination(SUNSET_SKY_ID);
    shell.play(20 * S, SUNSET_SKY, grind);
    shell.minerva.act(Action::OpenSearch);
    shell.minerva.search_query("blue moon");
    shell.minerva.act(Action::SearchNext);
    shell.minerva.act(Action::SearchEnter);
    shell.play(25 * S, SUNSET_SKY, grind);
    shell.minerva.act(Action::OpenSearch);
    shell.minerva.search_query("ore");
    shell.minerva.search_pick_item("04010000");
    shell.play(30 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    let replay = bug_report::replay_with(&recording, with);

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let said: Vec<String> = recording.timeline(0).into_iter().filter(|line| line.contains("search picked")).collect();
    assert_eq!(said.len(), 2, "{said:?}");
}

#[test]
fn the_report_holds_only_the_item_files_fingerprint() {
    let (report, recording) = run_with_item_cards().report();

    let fingerprints: Vec<&str> = recording
        .lines
        .iter()
        .flat_map(|line| &line.answers)
        .filter_map(|answer| match answer {
            Answer::Items { answer: Ok(fingerprint) } => Some(fingerprint.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fingerprints, [bug_report::item_fingerprint(&items().files())]);
    let names = zip_names(&report.zip);
    assert!(names.iter().all(|name| !name.ends_with("items.json") && !name.ends_with("cash_shop.json")), "{names:?}");
}

/// The game's client area as the shell captures it for an Item Card or Item Lookup press:
/// 1920x1200 at the screen's origin, each pixel's shade telling where it is, so a crop of
/// it can be told apart by its corner.
fn client_capture(pointer: ScreenPoint, at_ms: u64) -> ItemCapture {
    let img = image::RgbaImage::from_fn(1920, 1200, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 40, 255]));
    ItemCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: 0, y: 0 }, pointer }
}

/// The player pressed Ctrl+Shift+I over (900,600) at `at_ms`, and the capture read as
/// `name` (`None`: unreadable).
fn press_item_card(shell: &mut Shell, name: Option<&str>, at_ms: u64) {
    let pointer = ScreenPoint { x: 900, y: 600 };
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let area = ScreenRect { x: 916, y: 650, width: 200, height: 19 };
    let reading =
        ItemTooltipReading { tooltip: ItemTooltip::Found { area, name: name.map(String::from) }, pointer, at_ms };
    shell.minerva.on_item_capture_read(client_capture(pointer, at_ms), reading);
}

fn item_crops(recording: &Recording) -> Vec<&str> {
    recording.crops.iter().map(String::as_str).filter(|name| name.starts_with("crops/item-tooltip-")).collect()
}

#[test]
fn an_item_tooltip_that_couldnt_be_read_keeps_its_capture_around_the_pointer_for_the_run() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, None, 10 * S);
    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();

    let name = format!("crops/item-tooltip-{:010}@400,400.png", 10 * S);
    assert_eq!(item_crops(&recording), [name.as_str()]);
    let line = recording.lines.iter().find(|line| matches!(line.event, Event::ItemTooltip { .. })).expect("its line");
    assert_eq!(line.crop.as_deref(), Some(name.as_str()), "the line names its crop");
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (800, 800), "400 px each way from the pointer");
    assert_eq!(crop.get_pixel(0, 0).0, [(500 % 256) as u8, 200, 40, 255], "its corner at (500,200)");
}

#[test]
fn an_item_tooltip_read_as_no_item_keeps_its_capture_for_the_run() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, Some("Stlrge Wlng Zz"), 10 * S);
    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();

    assert_eq!(item_crops(&recording), [format!("crops/item-tooltip-{:010}@400,400.png", 10 * S)]);
}

#[test]
fn an_item_tooltip_read_as_an_item_keeps_its_capture_for_10_minutes() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, Some("Bronze Ore"), 10 * S);

    shell.play(5 * MIN, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(item_crops(&recording).len(), 1, "kept 10 minutes, as the EXP bar's are");

    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(item_crops(&recording), Vec::<&str>::new());
}

#[test]
fn a_capture_by_the_game_windows_edge_is_cropped_to_the_window() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let pointer = ScreenPoint { x: 1800, y: 100 };
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: 1500, y: 150, width: 200, height: 19 }, name: None };
    shell.minerva.on_item_capture_read(client_capture(pointer, 10 * S), ItemTooltipReading { tooltip, pointer, at_ms: 10 * S });

    let (report, recording) = shell.report();

    let name = format!("crops/item-tooltip-{:010}@400,100.png", 10 * S);
    assert_eq!(item_crops(&recording), [name.as_str()]);
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (520, 500), "from (1400,0) to the window's right edge, and 400 px down");
}

fn zip_bytes(zip: &[u8], name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    let mut bytes = Vec::new();
    archive.by_name(name).expect(name).read_to_end(&mut bytes).expect("bytes");
    bytes
}
