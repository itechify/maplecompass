//! Every minimap sample, fed through the core's public inputs, shows the title its
//! filename names: `street-name__map-name[--variant].png`.

use compass_core::{Compass, Frame, default_title_region};
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/minimap")
}

fn load(name: &str) -> image::RgbaImage {
    image::open(samples_dir().join(format!("{name}.png")))
        .unwrap_or_else(|e| panic!("sample {name}: {e}"))
        .to_rgba8()
}

fn frame(img: &image::RgbaImage, at_ms: u64) -> Frame {
    Frame { width: img.width(), height: img.height(), rgba: img.as_raw().clone(), at_ms }
}

/// The letters and digits of a name, lower-cased: `"Rocky Road I"` and the filename
/// part `"rocky-road-i"` both give `"rockyroadi"`.
///
/// The panel shows the raw reading for now, and ocrs sometimes drops the space before
/// a short last word ("Rocky RoadI"). Every letter must still be right. Once the panel
/// snaps to real map names (#4) these tests can compare names exactly.
fn letters(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// `--window` samples are whole game-window screenshots including the Windows title
/// bar (25–28 px, hand-cropped). The client area starts below it.
const TITLE_BAR_PX: u32 = 27;

/// Crop the default Title Region out of a window screenshot, the way the shell crops
/// it out of the game's client area.
fn default_region_frame(window: &image::RgbaImage, at_ms: u64) -> Frame {
    let client = image::imageops::crop_imm(
        window,
        0,
        TITLE_BAR_PX,
        window.width(),
        window.height() - TITLE_BAR_PX,
    )
    .to_image();
    let r = default_title_region(client.width(), client.height());
    let crop = image::imageops::crop_imm(&client, r.x, r.y, r.width, r.height).to_image();
    frame(&crop, at_ms)
}

/// Every sample, in one session: `--window` screenshots go through the default Title
/// Region (this covers the wide long-name minimap and the narrow one that must be
/// trimmed); other samples (a minimap crop, a live Title Region capture) are fed whole.
#[test]
fn every_sample_shows_the_street_name_and_map_name_its_filename_names() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 5, "expected the minimap samples, found {}", paths.len());

    let mut compass = Compass::new();
    let mut wrong = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let (answer, variant) = match name.split_once("--") {
            Some((a, v)) => (a, Some(v)),
            None => (name.as_str(), None),
        };
        let (street, map) = answer.split_once("__").expect("street__map filename");
        let img = load(&name);
        let at_ms = 500 * i as u64;
        let input =
            if variant == Some("window") { default_region_frame(&img, at_ms) } else { frame(&img, at_ms) };

        let shown = compass.on_frame(input).title.clone();

        let got = shown.as_ref().map(|t| (letters(&t.street_name), letters(&t.map_name)));
        if got != Some((letters(street), letters(map))) {
            wrong.push(format!("{name}: panel shows {shown:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong titles:\n{}", wrong.join("\n"));
}

#[test]
fn a_frame_without_the_minimap_shows_no_title() {
    let mut compass = Compass::new();
    let kerning = load("victoria-road__kerning-city--window");
    compass.on_frame(default_region_frame(&kerning, 0));

    // e.g. a loading screen: the Title Region is plain black.
    let black = image::RgbaImage::from_pixel(560, 96, image::Rgba([0, 0, 0, 255]));
    let state = compass.on_frame(frame(&black, 500));

    assert_eq!(state.title, None);
}
