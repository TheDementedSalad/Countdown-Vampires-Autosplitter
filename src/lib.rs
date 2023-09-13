#![no_std]
// #![feature(type_alias_impl_trait, const_async_blocks)]
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::undocumented_unsafe_blocks,
    rust_2018_idioms
)]

use asr::{
    emulator::ps1::Emulator,
    future::{next_tick, retry},
    time::Duration,
    time_util::frame_count,
    timer::{self, TimerState},
    watcher::Watcher,
};

asr::panic_handler!();
asr::async_main!(stable);

async fn main() {
    let settings = Settings::register();

    loop {
        // Hook to the target process
        let mut emulator = retry(|| Emulator::attach()).await;
        let mut watchers = Watchers::default();
        let offsets = Offsets::new();

        loop {
            if !emulator.is_open() {
                break;
            }

            if emulator.update() {
                // Splitting logic. Adapted from OG LiveSplit:
                // Order of execution
                // 1. update() will always be run first. There are no conditions on the execution of this action.
                // 2. If the timer is currently either running or paused, then the isLoading, gameTime, and reset actions will be run.
                // 3. If reset does not return true, then the split action will be run.
                // 4. If the timer is currently not running (and not paused), then the start action will be run.
                update_loop(&emulator, &offsets, &mut watchers);

                let timer_state = timer::state();
                if timer_state == TimerState::Running || timer_state == TimerState::Paused {
                    if let Some(is_loading) = is_loading(&watchers, &settings) {
                        if is_loading {
                            timer::pause_game_time()
                        } else {
                            timer::resume_game_time()
                        }
                    }

                    if let Some(game_time) = game_time(&watchers, &settings) {
                        timer::set_game_time(game_time)
                    }

                    if reset(&watchers, &settings) {
                        timer::reset()
                    } else if split(&watchers, &settings) {
                        timer::split()
                    }
                }

                if timer::state() == TimerState::NotRunning && start(&watchers, &settings) {
                    timer::start();
                    timer::pause_game_time();

                    if let Some(is_loading) = is_loading(&watchers, &settings) {
                        if is_loading {
                            timer::pause_game_time()
                        } else {
                            timer::resume_game_time()
                        }
                    }
                }
            }
            next_tick().await;
        }
    }
}

#[derive(asr::user_settings::Settings)]
struct Settings {
    #[default = true]
    /// ---------- Start Conditions Below ----------
    _condit: bool,

    #[default = true]
    /// START --> Enable auto start
    start: bool,

    #[default = true]
    /// ---------- End Split Below ----------
    _ending: bool,

    #[default = true]
    /// Splits on either Good End or Bad End
    end: bool,

    #[default = true]
    /// ---------- Door Splits Below ----------
    _doors: bool,

    #[default = false]
    /// Door splits - Will split on every room
    door_split: bool,

    #[default = true]
    /// ---------- Item Splits Below ----------
    _items: bool,

    #[default = false]
    /// Keno Ticket
    keno: bool,

    #[default = false]
    /// VIP Suzie Card
    susie: bool,

    #[default = false]
    /// VIP Nancy Card
    nancy: bool,

    #[default = false]
    /// VIP Cheryl Card
    cheryl: bool,

    #[default = false]
    /// Show Stage Key
    stagekey: bool,

    #[default = false]
    /// VIP Leagan Card
    leagan: bool,

    #[default = false]
    /// Attraction Key
    attract: bool,

    #[default = false]
    /// Museum Key
    museum: bool,

    #[default = false]
    /// Desert Moon Control Room Key
    moon: bool,

    #[default = false]
    /// Key to "Evil House"
    evil: bool,

    #[default = false]
    /// The Spear Key
    spear: bool,

    #[default = false]
    /// Card Disk C
    cardc: bool,

    #[default = false]
    /// Card Disk D
    cardd: bool,

    #[default = false]
    /// VIP Sydney Card
    sydney: bool,

    #[default = false]
    /// No.9 Playing Card
    card9: bool,

    #[default = false]
    /// Blue Clock Hand
    bluehand: bool,

    #[default = false]
    /// Red Clock Hand
    redhand: bool,

    #[default = false]
    /// Panel No.1
    panel1: bool,

    #[default = false]
    /// Event Room Key
    event: bool,

    #[default = false]
    /// Panel No.2
    panel2: bool,

    #[default = false]
    /// Panel No.4
    panel4: bool,

    #[default = false]
    /// Panel No.6
    panel6: bool,

    #[default = false]
    /// Y-Shaped Panel Key
    ykey: bool,

    #[default = false]
    /// Key to Passageway D-4
    d4: bool,

    #[default = false]
    /// Key to Shipping Area Parking Lot
    lot: bool,

    #[default = false]
    /// Key to Campground Vehicle
    camp: bool,

    #[default = false]
    /// Key to Small Storage Room
    small: bool,

    #[default = false]
    /// Forklift Key
    fork: bool,

    #[default = false]
    /// Log House Key
    log: bool,

    #[default = false]
    /// Key to the "Guesthouse"
    guest: bool,

    #[default = false]
    /// Shower Room Key
    shower: bool,

    #[default = false]
    /// Key to Chainsaw Shelf
    shelf: bool,

    #[default = false]
    /// Bourbon
    bourbon: bool,

    #[default = false]
    /// Marlintown Gate Key
    marlin: bool,

    #[default = false]
    /// Chainsaw
    chain: bool,

    #[default = false]
    /// Observation Room Key
    observ: bool,

    #[default = false]
    /// Sterilization Passageway Key
    sterile: bool,

    #[default = false]
    /// M82A1
    m8: bool,

    #[default = false]
    /// Code - SIN Key
    sin: bool,

    #[default = false]
    /// Fuse
    fuse: bool,
}

// Defines the watcher type of
#[derive(Default)]
struct Watchers {
    hp: Watcher<u16>,
    igt: Watcher<Duration>,
    map_id: Watcher<u16>,
    inventory: Watcher<[u16; 12]>,
    ending: Watcher<u16>,
    accumulated_igt: Duration,
    buffer_igt: Duration,
}

struct Offsets {
    gamecode_ntsc: u32,
    hp: u32,
    igt: u32,
    map_id: u32,
    item_1: u32,
    ending: u32,
}

// Offsets of data, relative to the beginning of the games VRAM
impl Offsets {
    fn new() -> Self {
        Self {
            gamecode_ntsc: 0x93DC,
            hp: 0xB3F2E,
            igt: 0xB3EFC,
            map_id: 0xB3EF2,
            item_1: 0xB3F42,
            ending: 0xB3F28,
        }
    }
}

fn update_loop(game: &Emulator, offsets: &Offsets, watchers: &mut Watchers) {
    match &game
        .read::<[u8; 11]>(offsets.gamecode_ntsc)
        .unwrap_or_default()
    {
        b"SLUS_008.98" | b"SLUS_011.99" => {
            // The gamecodes provided above ensure you are running the correct game
            watchers.hp.update(game.read::<u16>(offsets.hp).ok());
            watchers.igt.update_infallible(frame_count::<30>(
                game.read::<u32>(offsets.igt).unwrap_or_default() as _,
            ));
            watchers
                .map_id
                .update(game.read::<u16>(offsets.map_id).ok());
            watchers.inventory.update_infallible(
                game.read::<[[u16; 3]; 12]>(offsets.item_1)
                    .unwrap_or_default()
                    .map(|[item, _, _]| item),
            );
            watchers
                .ending
                .update(game.read::<u16>(offsets.ending).ok());
        }
        _ => {
            // If the emulator is loading the wrong game, the watchers will update to their default state
            watchers.hp.update_infallible(u16::default());
            watchers.igt.update_infallible(Duration::default());
            watchers.map_id.update_infallible(u16::default());
            watchers.inventory.update_infallible([u16::default(); 12]);
            watchers.ending.update_infallible(u16::default());
        }
    };


    // Reset the buffer IGT variables when the timer is stopped
    if timer::state() == TimerState::NotRunning {
        watchers.accumulated_igt = Duration::ZERO;
        watchers.buffer_igt = Duration::ZERO;
    }

    if let Some(igt) = &watchers.igt.pair {
        if igt.old > igt.current {
            watchers.accumulated_igt += igt.old - watchers.buffer_igt;
            watchers.buffer_igt = igt.current;
        }
    }
}

// If the setting "start" is not selected, nothing will happen
// Checks to see if the current IGT > 0 and the old IGT == 0
fn start(watchers: &Watchers, settings: &Settings) -> bool {
    if !settings.start {
        return false;
    }

    settings.start
        && watchers
            .igt
            .pair
            .is_some_and(|pair| pair.changed_from(&Duration::ZERO))
}

fn split(watchers: &Watchers, settings: &Settings) -> bool {
    if settings.door_split && watchers.map_id.pair.is_some_and(|i| i.changed()) {
        true
    } else if settings.end
        && watchers.ending.pair.is_some_and(|i| i.changed_to(&0xFFFF))
        && watchers
            .map_id
            .pair
            .is_some_and(|i| i.changed() && (i.current == 123 || i.current == 110))
    {
        true
    } else {
        watchers.inventory.pair.is_some_and(|inventory| {
        (settings.keno && inventory.check(|arr| arr.contains(&309)))
            || (settings.susie && inventory.check(|arr| arr.contains(&303)))
            || (settings.nancy && inventory.check(|arr| arr.contains(&304)))
            || (settings.cheryl && inventory.check(|arr| arr.contains(&302)))
            || (settings.stagekey && inventory.check(|arr| arr.contains(&310)))
            || (settings.leagan && inventory.check(|arr| arr.contains(&305)))
            || (settings.attract && inventory.check(|arr| arr.contains(&335)))
            || (settings.museum && inventory.check(|arr| arr.contains(&336)))
            || (settings.moon && inventory.check(|arr| arr.contains(&337)))
            || (settings.evil && inventory.check(|arr| arr.contains(&340)))
            || (settings.spear && inventory.check(|arr| arr.contains(&308)))
            || (settings.cardc && inventory.check(|arr| arr.contains(&338)))
            || (settings.cardd && inventory.check(|arr| arr.contains(&339)))
            || (settings.sydney && inventory.check(|arr| arr.contains(&306)))
            || (settings.card9 && inventory.check(|arr| arr.contains(&311)))
            || (settings.bluehand && inventory.check(|arr| arr.contains(&331)))
            || (settings.redhand && inventory.check(|arr| arr.contains(&332)))
            || (settings.panel1 && inventory.check(|arr| arr.contains(&359)))
            || (settings.event && inventory.check(|arr| arr.contains(&363)))
            || (settings.panel2 && inventory.check(|arr| arr.contains(&364)))
            || (settings.panel4 && inventory.check(|arr| arr.contains(&366)))
            || (settings.panel6 && inventory.check(|arr| arr.contains(&368)))
            || (settings.ykey && inventory.check(|arr| arr.contains(&343)))
            || (settings.d4 && inventory.check(|arr| arr.contains(&383)))
            || (settings.lot && inventory.check(|arr| arr.contains(&385)))
            || (settings.camp && inventory.check(|arr| arr.contains(&392)))
            || (settings.small && inventory.check(|arr| arr.contains(&393)))
            || (settings.fork && inventory.check(|arr| arr.contains(&434)))
            || (settings.log && inventory.check(|arr| arr.contains(&408)))
            || (settings.guest && inventory.check(|arr| arr.contains(&435)))
            || (settings.shower && inventory.check(|arr| arr.contains(&413)))
            || (settings.shelf && inventory.check(|arr| arr.contains(&403)))
            || (settings.bourbon && inventory.check(|arr| arr.contains(&415)))
            || (settings.marlin && inventory.check(|arr| arr.contains(&405)))
            || (settings.chain && inventory.check(|arr| arr.contains(&404)))
            || (settings.observ && inventory.check(|arr| arr.contains(&428)))
            || (settings.sterile && inventory.check(|arr| arr.contains(&429)))
            || (settings.m8 && inventory.check(|arr| arr.contains(&111)))
            || (settings.sin && inventory.check(|arr| arr.contains(&423)))
            || (settings.fuse && inventory.check(|arr| arr.contains(&430)))
        })
    }
}

fn reset(_watchers: &Watchers, _settings: &Settings) -> bool {
    false
}

// Some(true) is equivelant to "return true"
fn is_loading(_watchers: &Watchers, _settings: &Settings) -> Option<bool> {
    Some(true)
}

fn game_time(watchers: &Watchers, _settings: &Settings) -> Option<Duration> {
    Some(watchers.igt.pair?.current + watchers.accumulated_igt - watchers.buffer_igt)
}
