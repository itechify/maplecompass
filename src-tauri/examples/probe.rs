//! Read-only smoke test of the shell's adapters without any UI: find the "MapleStory"
//! window, capture its default Title Region a few times, and print what the recogniser
//! read (the street line and map line, before they are snapped to a map) and what the
//! core shows. It only reads window geometry and screen pixels (ADR 0001), and fetches
//! MeowDB's map index once (cached for a day, ADR 0003).
//!
//! `cargo run -p maplecompass --example probe [reads] [save.png]`
//! With a path, the last captured Title Region is saved there for inspection; as a
//! `--title-region` sample in `samples/minimap` it is read exactly as the probe read it.
//!
//! `cargo run -p maplecompass --example probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]`
//! Record mode: capture each named rectangle of the game's client area (physical px,
//! relative to its top-left corner) at its own rate, until stopped (Ctrl+C) or for `n`
//! seconds, saving every crop as `<out-dir>/<name>/<ms>.png`. `<ms>` is zero-padded
//! milliseconds since the recording started, so names sort in time order. For example
//! `exp-bar=1015,1150,200,22@1 pickup-log=1630,830,285,135@10`. Nothing is read or
//! fetched: it only finds the game window and copies screen pixels.

use compass_core::{Compass, GAME_WINDOW_TITLE, Rect, TitleReader};
use maplecompass_lib::{capture, capture_title_frame, gamewin, meowdb::MeowDbSource};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("record") {
        if let Err(e) = record(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]");
            std::process::exit(2);
        }
        return;
    }
    let mut args = args.into_iter();
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

/// How often record mode looks for the game window again, so crops follow it.
const FOLLOW_WINDOW: Duration = Duration::from_millis(250);
/// The fastest rate a region may be recorded at.
const MAX_HZ: f64 = 30.0;

/// One named rectangle of the client area, recorded at its own rate.
struct Track {
    name: String,
    /// Relative to the client area's top-left corner.
    rect: Rect,
    period: Duration,
    dir: PathBuf,
    /// When the next crop is due.
    next: Instant,
    saved: u64,
    /// Crops skipped because the recording fell behind (or the game window was gone).
    dropped: u64,
}

/// `name=x,y,w,h@hz`
fn parse_track(spec: &str, out: &Path, start: Instant) -> Result<Track, String> {
    let bad = || format!("bad region \"{spec}\": expected <name>=<x>,<y>,<w>,<h>@<hz>");
    let (name, rest) = spec.split_once('=').ok_or_else(bad)?;
    let (rect, hz) = rest.split_once('@').ok_or_else(bad)?;
    let nums: Vec<u32> = rect.split(',').map(|n| n.trim().parse()).collect::<Result<_, _>>().map_err(|_| bad())?;
    let [x, y, width, height] = nums[..] else { return Err(bad()) };
    let hz: f64 = hz.parse().map_err(|_| bad())?;
    let valid_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_name || width == 0 || height == 0 || !(hz > 0.0 && hz <= MAX_HZ) {
        return Err(format!("{}: names are letters, digits, - and _; sizes above 0; rates up to {MAX_HZ} Hz", bad()));
    }
    Ok(Track {
        name: name.into(),
        rect: Rect { x, y, width, height },
        period: Duration::from_secs_f64(1.0 / hz),
        dir: out.join(name),
        next: start,
        saved: 0,
        dropped: 0,
    })
}

fn record(args: &[String]) -> Result<(), String> {
    let (out, rest) = args.split_first().ok_or("no output directory")?;
    let out = PathBuf::from(out);
    let start = Instant::now();
    let mut seconds = None;
    let mut tracks = Vec::new();
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        if arg == "--seconds" {
            let n: f64 = rest.next().and_then(|s| s.parse().ok()).ok_or("--seconds needs a number")?;
            seconds = Some(Duration::from_secs_f64(n));
        } else {
            tracks.push(parse_track(arg, &out, start)?);
        }
    }
    if tracks.is_empty() {
        return Err("no regions to record".into());
    }

    gamewin::look_for(GAME_WINDOW_TITLE);
    let mut client = gamewin::find_game_client().ok_or(format!("No visible \"{GAME_WINDOW_TITLE}\" window found."))?;
    println!("Game client area: {client:?}");
    for t in &tracks {
        let r = t.rect;
        if r.x + r.width > client.width || r.y + r.height > client.height {
            return Err(format!("region {} doesn't fit the {}x{} client area", t.name, client.width, client.height));
        }
        std::fs::create_dir_all(&t.dir).map_err(|e| format!("{}: {e}", t.dir.display()))?;
    }

    // PNGs are encoded on their own thread, so a slow save never delays a capture.
    let (crops, to_save) = mpsc::channel::<(PathBuf, u32, u32, Vec<u8>)>();
    let writer = std::thread::spawn(move || {
        for (path, width, height, rgba) in to_save {
            let img = image::RgbaImage::from_raw(width, height, rgba).expect("crop size");
            // Written whole, then renamed: stopping mid-save leaves no broken PNG.
            let part = path.with_extension("png.part");
            if let Err(e) = img.save_with_format(&part, image::ImageFormat::Png).and_then(|_| {
                std::fs::rename(&part, &path).map_err(image::ImageError::IoError)
            }) {
                eprintln!("saving {} failed: {e}", path.display());
            }
        }
    });

    println!("Recording to {} (Ctrl+C to stop)", out.display());
    let mut next_follow = start + FOLLOW_WINDOW;
    let mut game_found = true;
    let mut next_report = start + Duration::from_secs(1);
    loop {
        let now = Instant::now();
        if seconds.is_some_and(|s| now - start >= s) {
            break;
        }
        if now >= next_follow {
            next_follow = now + FOLLOW_WINDOW;
            let found = gamewin::find_game_client();
            if found.is_some() != game_found {
                println!("{}", if found.is_some() { "Game window back." } else { "Game window gone: skipping crops." });
            }
            game_found = found.is_some();
            client = found.unwrap_or(client);
        }
        for t in tracks.iter_mut().filter(|t| now >= t.next) {
            // Behind by more than a period: those crops are dropped, not taken late.
            let behind = (now - t.next).as_secs_f64() / t.period.as_secs_f64();
            t.dropped += behind as u64;
            t.next += t.period.mul_f64(behind.floor() + 1.0);
            if !game_found {
                t.dropped += 1;
                continue;
            }
            let r = t.rect;
            let (x, y) = (client.x + r.x as i32, client.y + r.y as i32);
            match capture::capture_rgba(x, y, r.width as i32, r.height as i32) {
                Ok(rgba) => {
                    let ms = (now - start).as_millis();
                    let path = t.dir.join(format!("{ms:08}.png"));
                    let _ = crops.send((path, r.width, r.height, rgba));
                    t.saved += 1;
                }
                Err(e) => {
                    eprintln!("capture of {} failed: {e}", t.name);
                    t.dropped += 1;
                }
            }
        }
        if now >= next_report {
            next_report += Duration::from_secs(1);
            let counts: Vec<String> =
                tracks.iter().map(|t| format!("{} {} (dropped {})", t.name, t.saved, t.dropped)).collect();
            println!("{:>4} s  {}", (now - start).as_secs(), counts.join("  "));
        }
        let due = tracks.iter().map(|t| t.next).chain([next_follow, next_report]).min().unwrap_or(now);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    drop(crops);
    let _ = writer.join();
    for t in &tracks {
        println!("{}: {} crops saved, {} dropped", t.name, t.saved, t.dropped);
    }
    Ok(())
}
