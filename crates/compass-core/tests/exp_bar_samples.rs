//! Every EXP bar sample, fed through the core's public inputs as an EXP Bar Region
//! frame, reads as what it shows, or as unreadable; never as a wrong number.
//!
//! - Single samples are named for their reading: `<exp>__<percent>[--variant].png`
//!   ("160__3.81--kerning-city-window"), or `unreadable[--variant].png`. The
//!   `--window` ones are client (1005, h-56) 240x32 cropped from the window samples in
//!   `samples/minimap`.
//! - `recording-*` folders are the probe's record mode (one crop a second, named by
//!   capture time in ms) with their readings in `readings.json` (`null`: unreadable).
//!   2026-10-07: 150 s of hunting at Lv 15, client (1005,1144) 240x32 of a 1920x1200
//!   client. Its readings all fit one level total of ~16,682 EXP (the game rounds the
//!   percentage), which is how the frames ocrs misread were caught.

mod common;

use common::FakeSource;
use compass_core::{
    Compass, ExpBar, ExpBarRegion, Frame, GameWindow, Rect, ScreenRect, Settings, Size, TitleRegion,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/exp-bar")
}

fn frame(path: &Path) -> Frame {
    let img = image::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())).to_rgba8();
    common::frame(&img, 0)
}

/// What the panel shows for one EXP Bar Region frame, read by a core set up with EXP
/// tracking on (a fresh one, so no earlier reading lingers).
fn read(frame: Frame) -> Option<ExpBar> {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(FakeSource::offline());
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    compass.on_exp_bar_frame(frame).exp_bar
}

/// "160__3.81" -> 160 EXP at 381 hundredths; "unreadable" -> `None`.
fn named_reading(name: &str) -> Option<ExpBar> {
    let answer = name.split_once("--").map_or(name, |(a, _)| a);
    if answer == "unreadable" {
        return None;
    }
    let (exp, percent) = answer.split_once("__").expect("<exp>__<percent> or unreadable");
    let (whole, hundredths) = percent.split_once('.').expect("a percentage with two decimals");
    Some(ExpBar {
        exp: exp.parse().expect("exp"),
        percent_hundredths: whole.parse::<u32>().expect("percent") * 100 + hundredths.parse::<u32>().expect("percent"),
    })
}

#[test]
fn every_single_sample_reads_as_its_filename_names() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 3, "expected the EXP bar samples, found {}", paths.len());

    let mut wrong = Vec::new();
    for path in &paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let shown = read(frame(path));
        if shown != named_reading(&name) {
            wrong.push(format!("{name}: panel shows {shown:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    exp: u64,
    percent_hundredths: u32,
}

#[test]
fn every_recorded_frame_reads_as_its_readings_file_says() {
    let recordings: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    assert!(!recordings.is_empty(), "expected a recording");

    let mut wrong = Vec::new();
    for dir in &recordings {
        let json = std::fs::read_to_string(dir.join("readings.json")).expect("readings.json");
        let expected: BTreeMap<String, Option<Expected>> = serde_json::from_str(&json).expect("readings json");
        let pngs = std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "png"));
        assert_eq!(pngs.count(), expected.len(), "{}: a reading for every crop", dir.display());
        for (file, want) in expected {
            let want = want.map(|e| ExpBar { exp: e.exp, percent_hundredths: e.percent_hundredths });
            let shown = read(frame(&dir.join(&file)));
            if shown != want {
                wrong.push(format!("{}/{file}: panel shows {shown:?}, expected {want:?}", dir.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}
