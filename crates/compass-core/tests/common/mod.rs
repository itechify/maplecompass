//! Shared test inputs: minimap samples turned into frames, and a fake data source serving
//! made-up worlds in OSMS's shape (ADR 0008), with MeowDB's saved mob pages and search
//! answers (personal-use test data, ADR 0003) until #58 and #62.
#![allow(dead_code)]

pub mod osms;
pub mod player;
pub mod world;

use compass_core::osms::OsmsFiles;
use compass_core::{
    Compass, ExpBarRegion, Fetched, Frame, GameWindow, ItemHit, MapDataSource, MobDetails,
    PanelState, Rect, ScreenRect, Settings, Size, SourceError, TitleRegion, default_title_region, meowdb,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

pub fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/minimap")
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn load(name: &str) -> image::RgbaImage {
    image::open(samples_dir().join(format!("{name}.png")))
        .unwrap_or_else(|e| panic!("sample {name}: {e}"))
        .to_rgba8()
}

pub fn frame(img: &image::RgbaImage, at_ms: u64) -> Frame {
    Frame { width: img.width(), height: img.height(), rgba: img.as_raw().clone(), at_ms }
}

/// A loading screen: the Title Region is plain black.
pub fn black_frame(at_ms: u64) -> Frame {
    frame(&image::RgbaImage::from_pixel(560, 96, image::Rgba([0, 0, 0, 255])), at_ms)
}

/// `--window` samples are whole game-window screenshots including the Windows title
/// bar (25–28 px, hand-cropped). The client area starts below it.
const TITLE_BAR_PX: u32 = 27;

/// The game's client area in a window screenshot: everything below the title bar.
pub fn client_of(window: &image::RgbaImage) -> image::RgbaImage {
    image::imageops::crop_imm(window, 0, TITLE_BAR_PX, window.width(), window.height() - TITLE_BAR_PX)
        .to_image()
}

/// Capture `region` (physical screen px) from a game client area shown on screen at
/// `game`, the way the shell captures the panel state's Title Region.
pub fn capture(client: &image::RgbaImage, game: ScreenRect, region: ScreenRect, at_ms: u64) -> Frame {
    let (x, y) = ((region.x - game.x) as u32, (region.y - game.y) as u32);
    let crop = image::imageops::crop_imm(client, x, y, region.width, region.height).to_image();
    assert_eq!(crop.dimensions(), (region.width, region.height), "region inside the client area");
    frame(&crop, at_ms)
}

/// Crop the default Title Region out of a window screenshot, the way the shell crops
/// it out of the game's client area.
pub fn default_region_frame(window: &image::RgbaImage, at_ms: u64) -> Frame {
    let client = client_of(window);
    let r = default_title_region(client.width(), client.height());
    let crop = image::imageops::crop_imm(&client, r.x, r.y, r.width, r.height).to_image();
    frame(&crop, at_ms)
}

/// The frame the shell would capture for a sample: `--window` samples go through the
/// default Title Region; other samples (a minimap crop, a live Title Region capture)
/// are fed whole.
pub fn sample_frame(name: &str, at_ms: u64) -> Frame {
    let img = load(name);
    if name.ends_with("--window") { default_region_frame(&img, at_ms) } else { frame(&img, at_ms) }
}

/// Two confident reads of a sample: the panel switches to its map.
pub fn arrive(compass: &mut Compass, sample: &str, at_ms: u64) -> PanelState {
    compass.on_frame(sample_frame(sample, at_ms));
    compass.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

/// A core with EXP tracking on: both regions kept, on a 1920x1200 game window.
pub fn tracking() -> Compass {
    tracking_with(FakeSource::osms())
}

/// As [`tracking`], with maps and their pages from `source`.
pub fn tracking_with(source: FakeSource) -> Compass {
    let client = Size { width: 1920, height: 1200 };
    let mut compass = Compass::new(source);
    compass.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        exp_bar_region: Some(ExpBarRegion { marked: Rect { x: 1005, y: 1144, width: 240, height: 32 }, client }),
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    compass.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    compass
}

/// A Title Region marked around the minimap title of the Kerning City window sample,
/// whose client area is 1916x1197.
pub fn kerning_title_region() -> TitleRegion {
    TitleRegion {
        marked: Rect { x: 70, y: 40, width: 130, height: 48 },
        client: Size { width: 1916, height: 1197 },
    }
}

/// Sunset Sky's and the Construction Site's map IDs: both have a Map Summary in the
/// made-up game world ([`osms::game`]).
pub const SUNSET_SKY_ID: &str = "010003070";
pub const CONSTRUCTION_SITE_ID: &str = "010003080";

/// The made-up game world, with the Construction Site and Sunset Sky sharing the Kerning
/// City sample's title: the Construction Site, listed first, shows first.
pub fn sharing_a_title() -> FakeSource {
    FakeSource::with_world(osms::game().titled("Victoria Road", "Kerning City", &[osms::CONSTRUCTION_SITE, osms::SUNSET_SKY]))
}

/// The Northern Top of the Construction Site's map ID: it has a Map Summary.
pub const NORTHERN_TOP_ID: &str = "010003081";

/// As [`sharing_a_title`], with the Northern Top of the Construction Site sharing the
/// title too: three Candidates in map list order, the Northern Top first (shown first),
/// then the Construction Site, then Sunset Sky.
pub fn three_sharing_a_title() -> FakeSource {
    let ids = [osms::NORTHERN_TOP, osms::CONSTRUCTION_SITE, osms::SUNSET_SKY];
    FakeSource::with_world(osms::game().titled("Victoria Road", "Kerning City", &ids))
}

/// A MeowDB mob page saved on 2026-10-08:
/// `tests/fixtures/meowdb-monsters/<7-digit id>__<slug>.html`.
pub fn saved_mob_page(id: &str) -> Option<String> {
    let dir = fixtures_dir().join("meowdb-monsters");
    let file = std::fs::read_dir(dir)
        .expect("fixture dir")
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(&format!("{id:0>7}__")))?;
    Some(std::fs::read_to_string(file.path()).expect("fixture"))
}

/// A file-name form of a name, as the probe names samples: lower case, apostrophes
/// dropped, anything else but letters and digits as one `-` ("Hero's Gladius" ->
/// "heros-gladius").
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().filter(|c| !matches!(c, '\'' | '’')) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// MeowDB's search's answer for `name`, saved on 2026-10-09:
/// `tests/fixtures/meowdb-search/<slug>.json`.
pub fn saved_search(name: &str) -> Option<String> {
    std::fs::read_to_string(fixtures_dir().join("meowdb-search").join(format!("{}.json", slug(name)))).ok()
}

/// The ID of every saved mob page, in file name order.
pub fn saved_mob_ids() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("meowdb-monsters"))
        .expect("fixture dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.iter().map(|n| n[..n.find("__").expect("<id>__<slug>")].trim_start_matches('0').to_string()).collect()
}

type MobAnswer = Result<MobDetails, SourceError>;
type ImageAnswer = Result<String, SourceError>;
type SearchAnswer = Result<Vec<ItemHit>, SourceError>;
pub type OsmsAnswer = Poll<Result<Fetched<OsmsFiles>, SourceError>>;

/// A data source that answers from memory and counts what it is asked.
pub struct FakeSource {
    /// Answers for successive asks for OSMS's file set; the last one repeats.
    osms_answers: Vec<OsmsAnswer>,
    /// How many times OSMS's file set was asked for.
    pub osms_requests: Arc<AtomicU32>,
    /// The sites are unreachable and the answers are copies saved at this time (ms since
    /// the Unix epoch); `None`: they are fresh.
    saved_at: Option<u64>,
    /// Answers a mob page request for a mob ID. Defaults to the saved mob pages, read
    /// with the core's MeowDB reader; a mob without a saved page is unreachable.
    mob_pages: Box<dyn FnMut(&str) -> MobAnswer + Send>,
    /// Answers a mob image request for a mob ID. Defaults to `"image:<id>"`.
    images: Box<dyn FnMut(&str) -> ImageAnswer + Send>,
    /// The mob ID of every mob page fetch started, in order.
    pub mob_requests: Arc<Mutex<Vec<String>>>,
    /// The mob ID of every mob image fetch started, in order.
    pub image_requests: Arc<Mutex<Vec<String>>>,
    /// How many polls a mob page or image answer takes: 0 when it's in the cache (answered
    /// at once), otherwise it arrives on the `n`th poll after its fetch started, as from
    /// the network.
    mob_polls: usize,
    /// The mob page fetch in flight, its answer, and how many more polls it takes.
    mob_in_flight: Option<(String, MobAnswer, usize)>,
    image_in_flight: Option<(String, ImageAnswer, usize)>,
    /// Answers an item search for a name. Defaults to MeowDB's saved answers, read with the
    /// core's MeowDB reader; a name without a saved answer is unreachable.
    item_searches: Box<dyn FnMut(&str) -> SearchAnswer + Send>,
    /// The name of every item search started, in order.
    pub item_requests: Arc<Mutex<Vec<String>>>,
    /// How many polls an item search takes, as `mob_polls` for mob pages.
    item_polls: usize,
    item_in_flight: Option<(String, SearchAnswer, usize)>,
}

impl FakeSource {
    /// OSMS's file set answers these, one ask after another; the last one repeats.
    pub fn answering_osms(answers: Vec<OsmsAnswer>) -> Self {
        Self {
            osms_answers: answers,
            osms_requests: Arc::default(),
            saved_at: None,
            mob_pages: Box::new(read_saved_mob_page),
            images: Box::new(|id| Ok(format!("image:{id}"))),
            mob_requests: Arc::default(),
            image_requests: Arc::default(),
            mob_polls: 0,
            mob_in_flight: None,
            image_in_flight: None,
            item_searches: Box::new(read_saved_search),
            item_requests: Arc::default(),
            item_polls: 0,
            item_in_flight: None,
        }
    }

    /// OSMS serves this made-up world.
    pub fn with_world(world: osms::World) -> Self {
        Self::answering_osms(vec![fresh(world)])
    }

    /// OSMS serves the game's map list, with made-up figures on the maps the minimap
    /// samples show ([`osms::game`]).
    pub fn osms() -> Self {
        Self::with_world(osms::game())
    }

    /// OSMS can't be reached, and no set is saved.
    pub fn osms_offline() -> Self {
        Self::answering_osms(vec![Poll::Ready(Err(unreachable()))])
    }

    /// The sites can't be reached, and every answer is a copy saved at
    /// `saved_at_unix_ms`: OSMS's file set, mob pages and item searches.
    pub fn serving_saved_copies(self, saved_at_unix_ms: u64) -> Self {
        Self { saved_at: Some(saved_at_unix_ms), ..self }
    }

    fn fetched<T>(&self, answer: Poll<Result<T, SourceError>>) -> Poll<Result<Fetched<T>, SourceError>> {
        answer.map(|result| {
            result.map(|data| match self.saved_at {
                Some(saved_at_unix_ms) => Fetched::SavedCopy { data, saved_at_unix_ms },
                None => Fetched::Fresh(data),
            })
        })
    }

    /// Answer mob page requests with `pages` instead of the saved mob pages.
    pub fn with_mob_pages(mut self, pages: impl FnMut(&str) -> MobAnswer + Send + 'static) -> Self {
        self.mob_pages = Box::new(pages);
        self
    }

    /// Serve `html` as the MeowDB page of mob `id`, and the saved pages for other mobs.
    pub fn with_mob_page(self, id: &str, html: String) -> Self {
        let id = id.to_string();
        self.with_mob_pages(move |asked| {
            if asked == id { meowdb::read_mob_page(&html) } else { read_saved_mob_page(asked) }
        })
    }

    /// Answer mob image requests with `images`.
    pub fn with_images(mut self, images: impl FnMut(&str) -> ImageAnswer + Send + 'static) -> Self {
        self.images = Box::new(images);
        self
    }

    /// Mob pages and images come over the network, not from the cache: each arrives on
    /// the poll after its fetch started.
    pub fn mobs_uncached(self) -> Self {
        self.mobs_taking(1)
    }

    /// Mob pages and images come over a slow network: each arrives on the `polls`th poll
    /// after its fetch started.
    pub fn mobs_taking(self, polls: usize) -> Self {
        Self { mob_polls: polls, ..self }
    }

    /// Answer item searches with `searches` instead of MeowDB's saved answers.
    pub fn with_item_searches(mut self, searches: impl FnMut(&str) -> SearchAnswer + Send + 'static) -> Self {
        self.item_searches = Box::new(searches);
        self
    }

    /// Item searches come over the network, not from the cache: each arrives on the
    /// `polls`th poll after it started.
    pub fn items_taking(self, polls: usize) -> Self {
        Self { item_polls: polls, ..self }
    }
}

fn unreachable() -> SourceError {
    SourceError::Unreachable("offline".into())
}

/// How many of `requests` (a [`FakeSource`]'s log) were for `id`.
pub fn requests_for(requests: &Mutex<Vec<String>>, id: &str) -> usize {
    requests.lock().unwrap().iter().filter(|r| *r == id).count()
}

fn read_saved_mob_page(id: &str) -> MobAnswer {
    match saved_mob_page(id) {
        Some(html) => meowdb::read_mob_page(&html),
        None => Err(SourceError::Unreachable(format!("no saved page for mob {id}"))),
    }
}

/// One fetch at a time: the answer for `id` if its fetch is the one in flight and it has
/// arrived, otherwise a new fetch, logged in `requests`, answered at once or (with
/// `polls` above 0) on the `polls`th poll after.
fn one_at_a_time<T>(
    id: &str,
    in_flight: &mut Option<(String, Result<T, SourceError>, usize)>,
    requests: &Mutex<Vec<String>>,
    polls: usize,
    answer: impl FnOnce(&str) -> Result<T, SourceError>,
) -> Poll<Result<T, SourceError>> {
    if let Some((asked, arrived, left)) = in_flight.take() {
        if asked == id {
            if left <= 1 {
                return Poll::Ready(arrived);
            }
            *in_flight = Some((asked, arrived, left - 1));
            return Poll::Pending;
        }
    }
    requests.lock().unwrap().push(id.to_string());
    let answer = answer(id);
    if polls > 0 {
        *in_flight = Some((id.to_string(), answer, polls));
        return Poll::Pending;
    }
    Poll::Ready(answer)
}

fn read_saved_search(name: &str) -> SearchAnswer {
    match saved_search(name) {
        Some(json) => meowdb::read_item_search(&json),
        None => Err(SourceError::Unreachable(format!("no saved search for {name:?}"))),
    }
}

/// A fresh answer of OSMS's file set: `world`'s files.
pub fn fresh(world: osms::World) -> OsmsAnswer {
    Poll::Ready(Ok(Fetched::Fresh(world.files())))
}

/// A fresh answer of OSMS's file set: these files, as they are.
pub fn fresh_files(files: OsmsFiles) -> OsmsAnswer {
    Poll::Ready(Ok(Fetched::Fresh(files)))
}

/// A saved copy of `world`'s files, saved at `saved_at_unix_ms`.
pub fn saved_copy(world: osms::World, saved_at_unix_ms: u64) -> OsmsAnswer {
    Poll::Ready(Ok(Fetched::SavedCopy { data: world.files(), saved_at_unix_ms }))
}

impl MapDataSource for FakeSource {
    fn osms_files(&mut self) -> Poll<Result<Fetched<OsmsFiles>, SourceError>> {
        let n = self.osms_requests.fetch_add(1, Ordering::SeqCst) as usize;
        let answer = self.osms_answers[n.min(self.osms_answers.len() - 1)].clone();
        match (answer, self.saved_at) {
            (Poll::Ready(Ok(Fetched::Fresh(data))), Some(saved_at_unix_ms)) => {
                Poll::Ready(Ok(Fetched::SavedCopy { data, saved_at_unix_ms }))
            }
            (answer, _) => answer,
        }
    }

    fn mob_page(&mut self, id: &str) -> Poll<Result<Fetched<MobDetails>, SourceError>> {
        let answer =
            one_at_a_time(id, &mut self.mob_in_flight, &self.mob_requests, self.mob_polls, &mut self.mob_pages);
        self.fetched(answer)
    }

    fn mob_image(&mut self, id: &str) -> Poll<Result<String, SourceError>> {
        one_at_a_time(id, &mut self.image_in_flight, &self.image_requests, self.mob_polls, &mut self.images)
    }

    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>> {
        let answer =
            one_at_a_time(name, &mut self.item_in_flight, &self.item_requests, self.item_polls, &mut self.item_searches);
        self.fetched(answer)
    }
}
