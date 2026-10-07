//! Read-only smoke test of the shell's adapters without any UI: find the "MapleStory"
//! window, capture its default Title Region a few times, and print what the recogniser
//! read (the street line and map line, before they are snapped to a map) and what the
//! core shows. It only reads window geometry and screen pixels (ADR 0001), and fetches
//! MeowDB's map index once (cached for a day, ADR 0003).
//!
//! `cargo run -p maplecompass --example probe [reads] [save.png]`
//! With a path, the last captured Title Region is saved there for inspection; as a
//! `--title-region` sample in `samples/minimap` it is read exactly as the probe read it.

use compass_core::{Compass, TitleReader};
use maplecompass_lib::{capture_title_frame, gamewin, meowdb::MeowDbSource};
use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let reads: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(5);
    let save = args.next();
    // Its own cache, outside the app's: the probe has no Tauri path resolver.
    let cache = std::env::temp_dir().join("maplecompass-probe-cache");
    let mut compass = Compass::new(MeowDbSource::new(cache).expect("HTTP client"));
    let reader = TitleReader::bundled();
    let title = compass.panel_state().game_window_title.clone();
    gamewin::look_for(&title);
    let Some(client) = gamewin::find_game_client() else {
        println!("No visible \"{title}\" window found.");
        return;
    };
    println!("Game client area: {client:?}");
    let start = Instant::now();
    let mut last = None;
    for _ in 0..reads {
        let t = Instant::now();
        match capture_title_frame(client, start.elapsed().as_millis() as u64) {
            Ok(frame) => {
                let reading = reader.read(&frame);
                last = Some(frame);
                match &reading.lines {
                    Some(lines) => println!("{:>4} ms  read {:?} / {:?}", t.elapsed().as_millis(), lines.street, lines.map),
                    None => println!("{:>4} ms  read nothing (unreadable)", t.elapsed().as_millis()),
                }
                let state = compass.on_reading(reading);
                println!("         {:?} {:?} not sure: {}", state.recognition, state.map, state.not_sure);
            }
            Err(e) => println!("capture failed: {e}"),
        }
        std::thread::sleep(Duration::from_millis(333));
    }
    if let (Some(path), Some(f)) = (save, last) {
        let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba).expect("frame size");
        img.save(&path).expect("save png");
        println!("saved {path}");
    }
}
