//! The player hunting with EXP tracking on, for the Personal Best: the EXP bar read every
//! 10 s, the panel confirming maps, the Grind Tracker's buttons pressed, and, if a test
//! asks, the shell's saves of the bests followed after each.

use compass_core::{Compass, ExpBar, ExpBarReading, PanelControl, PanelState, SavedBests, VisitBest};

const S: u64 = 1000;

/// The wall clock at capture time 0 when the shell tells it: 9 Oct 2026, 12:00 UTC.
pub const WALL_AT_START: u64 = 1_791_547_200_000;

/// The percentage the game shows for `exp` on a 20,000 EXP level, rounded to hundredths.
fn percent(exp: u64) -> u32 {
    ((exp * 20_000 + 20_000) / (2 * 20_000)) as u32
}

/// The player hunting on a 20,000 EXP level, the EXP bar read every 10 s. Readings at
/// whole half minutes of capture time are where the Grind Tracker's figures refresh.
pub struct Player {
    pub compass: Compass,
    pub exp: u64,
    pub at: u64,
    /// The wall clock at capture time 0, if the shell tells it.
    wall_at_start: Option<u64>,
    /// The shell's saves of the Personal Bests, if the test follows them.
    shell: Option<Shell>,
}

impl Player {
    /// The bar read at 1,000 [5.00%] at 0 s, then 10 EXP more every 10 s on a map the
    /// panel doesn't know, up to 30 s: the Session has started.
    pub fn new(compass: Compass) -> Self {
        Self::start(compass, None, None)
    }

    /// As [`Player::new`], with the shell telling the wall clock before every reading, as
    /// it does: [`WALL_AT_START`] at capture time 0, running with it.
    pub fn clocked(compass: Compass) -> Self {
        Self::clocked_at(compass, WALL_AT_START)
    }

    /// As [`Player::clocked`], the wall clock at `wall_at_start` at capture time 0: a
    /// later launch.
    pub fn clocked_at(compass: Compass, wall_at_start: u64) -> Self {
        Self::start(compass, Some(wall_at_start), None)
    }

    /// As [`Player::clocked`], following the shell's saves of the Personal Bests from
    /// the start.
    pub fn saved_by_the_shell(compass: Compass) -> Self {
        let shell = Shell::new(&compass);
        Self::start(compass, Some(WALL_AT_START), Some(shell))
    }

    fn start(compass: Compass, wall_at_start: Option<u64>, shell: Option<Shell>) -> Self {
        let mut player = Player { compass, exp: 1000, at: 0, wall_at_start, shell };
        player.read();
        player.hunt(30 * S, 10);
        player
    }

    /// The wall clock now, as the shell last told it.
    pub fn wall_clock(&self) -> Option<u64> {
        self.wall_at_start.map(|start| start + self.at)
    }

    /// How many times the shell has saved the bests.
    pub fn saves(&self) -> u32 {
        self.shell.as_ref().map_or(0, |shell| shell.saves)
    }

    /// The shell's loop, after an input: the panel state then.
    fn shell_loop(&mut self, input: &str) -> PanelState {
        if let Some(shell) = &mut self.shell {
            shell.after(&self.compass, &format!("{input} at {} ms", self.at));
        }
        self.compass.panel_state().clone()
    }

    pub fn read(&mut self) -> PanelState {
        if let Some(now) = self.wall_clock() {
            self.compass.on_wall_clock(now);
        }
        let bar = Some(ExpBar { exp: self.exp, percent_hundredths: percent(self.exp) });
        self.compass.on_exp_bar_reading(ExpBarReading { bar, at_ms: self.at });
        self.shell_loop("a reading")
    }

    /// The panel confirms the map of a sample: a Visit starts now.
    pub fn arrive(&mut self, sample: &str) -> PanelState {
        super::arrive(&mut self.compass, sample, self.at);
        self.shell_loop(sample)
    }

    /// Hunt for `ms` more, `gain` EXP every 10 s (`gain` × 360 EXP/h): the panel state at
    /// the last reading.
    pub fn hunt(&mut self, ms: u64, gain: u64) -> PanelState {
        let mut state = self.compass.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.exp += gain;
            state = self.read();
        }
        state
    }

    /// The bar doesn't read whole for `ms` more, a frame every 10 s: the panel state at
    /// the last.
    pub fn unread(&mut self, ms: u64) -> PanelState {
        let mut state = self.compass.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.compass.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: self.at });
            state = self.shell_loop("the bar unread");
        }
        state
    }

    /// Choose the next of the Candidates with the hotkey.
    pub fn choose_next(&mut self) -> PanelState {
        self.compass.on_hotkey("ctrl+shift+n");
        self.shell_loop("choosing the next Candidate")
    }

    /// Press a control on the panel or the Grind Tracker.
    pub fn press(&mut self, control: PanelControl) -> PanelState {
        self.compass.press(control);
        self.shell_loop(&format!("{control:?}"))
    }

    /// Remove the Visit whose best was `visit` from `map`'s, in the Personal Bests window.
    pub fn remove_visit(&mut self, map: &str, visit: VisitBest) -> PanelState {
        self.compass.remove_visit(map, visit);
        self.shell_loop(&format!("removing {visit:?} from {map}"))
    }
}

/// The shell's saves of the Personal Bests, as its loop makes them: whenever the bests'
/// revision has changed since the last.
struct Shell {
    revision: u64,
    saved: SavedBests,
    saves: u32,
}

impl Shell {
    /// Just after startup, with what was restored.
    fn new(compass: &Compass) -> Self {
        Self { revision: compass.bests_revision(), saved: compass.bests(), saves: 0 }
    }

    /// The shell's loop after `input`: it saves if the revision changed, which it must
    /// exactly when the bests did.
    fn after(&mut self, compass: &Compass, input: &str) {
        let (revision, bests) = (compass.bests_revision(), compass.bests());
        let changed = revision != self.revision;
        assert_eq!(changed, bests != self.saved, "{input}: saved {:?}, now {bests:?}", self.saved);
        if changed {
            *self = Self { revision, saved: bests, saves: self.saves + 1 };
        }
    }
}
