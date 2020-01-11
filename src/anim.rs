use std::collections::VecDeque;
use std::f64::consts::FRAC_PI_2;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};

use crate::Direction;
use crate::net::{Message, MetaMessage};

/// Tracks state of the target stripe animation
pub struct TargetStripeState {
    pub offset: f64,
}

impl TargetStripeState {
    const LENGTH: f64 = 2.0;

    fn new() -> TargetStripeState {
        TargetStripeState { offset: 0.0 }
    }

    fn advance_by(&mut self, ticks: f64) {
        self.offset = (self.offset + ticks) % Self::LENGTH;
    }

    pub fn pct_offset(&self) -> f64 {
        self.offset / Self::LENGTH
    }
}

/// Checks the direction in which the tile rotate animation spins
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RotateDir {
    /// Clockwise
    CW,
    /// Counterclockwise
    CCW,
}

/// Tracks state of the loose tile rotate animation
pub struct LooseRotateState {
    pub angle: f64,
}

impl LooseRotateState {
    const LENGTH: f64 = 0.25;

    fn new() -> LooseRotateState {
        LooseRotateState { angle: 0.0 }
    }

    fn reset(&mut self, dir: RotateDir) {
        self.angle += match dir {
            RotateDir::CW => -FRAC_PI_2,
            RotateDir::CCW => FRAC_PI_2,
        };
    }

    fn advance_by(&mut self, ticks: f64) {
        if self.angle == 0.0 {
            return;
        }
        let delta = FRAC_PI_2 / Self::LENGTH;
        let (delta, clamp): (f64, fn(f64, f64) -> f64) = if self.angle.is_sign_positive() {
            (-delta, f64::max)
        } else {
            (delta, f64::min)
        };
        self.angle = clamp(self.angle + delta * ticks, 0.0);
    }
}

/// Tracks state of loose tile insert animation
pub struct LooseInsertState {
    /// Direction in which the tiles are currently offset
    /// (same as the edge on which the loose tile started)
    pub offset_dir: Direction,
    /// Fraction of a tile remaining in the animation
    pub distance_left: f64,
    /// Row/column of the offset tiles
    coordinate: usize,
}

impl LooseInsertState {
    const LENGTH: f64 = 0.25;

    fn new() -> LooseInsertState {
        LooseInsertState {
            offset_dir: Direction::North,
            distance_left: 0.0,
            coordinate: 0,
        }
    }

    fn reset(&mut self, dir: Direction, coord: usize) {
        self.offset_dir = dir;
        self.distance_left = 1.0;
        self.coordinate = coord;
    }

    fn advance_by(&mut self, ticks: f64) {
        if self.distance_left == 0.0 {
            return;
        }
        self.distance_left = (self.distance_left - ticks / Self::LENGTH).max(0.0);
    }

    pub fn applies_to_pos(&self, (row, col): (usize, usize)) -> bool {
        if self.distance_left == 0.0 {
            return false;
        }
        let should_be_coord = match self.offset_dir {
            Direction::North | Direction::South => col,
            Direction::East | Direction::West => row,
        };
        should_be_coord == self.coordinate
    }

    pub fn applies_to_loose(&self, (dir, guide_idx): (Direction, usize)) -> bool {
        if self.distance_left == 0.0 {
            return false;
        }
        if dir == self.offset_dir || dir == self.offset_dir * Direction::South {
            self.coordinate == 2 * guide_idx + 1
        } else {
            false
        }
    }
}

/// Tracks state of all currently running animations
pub struct AnimGlobalState {
    pub target_stripe: TargetStripeState,
    pub loose_rotate: LooseRotateState,
    pub loose_insert: LooseInsertState,
    net_queue: Option<Arc<Mutex<VecDeque<MetaMessage>>>>,
}

impl AnimGlobalState {
    fn new() -> AnimGlobalState {
        AnimGlobalState {
            target_stripe: TargetStripeState::new(),
            loose_rotate: LooseRotateState::new(),
            loose_insert: LooseInsertState::new(),
            net_queue: None,
        }
    }

    pub fn advance_by(&mut self, ticks: f64) {
        self.target_stripe.advance_by(ticks);
        self.loose_rotate.advance_by(ticks);
        self.loose_insert.advance_by(ticks);
    }

    pub fn set_send(&mut self, send: Arc<Mutex<VecDeque<MetaMessage>>>) {
        self.net_queue = Some(send)
    }

    pub fn apply(&mut self, msg: AnimSync) {
        match msg {
            AnimSync::Rotate(dir) => self.loose_rotate.reset(dir),
            AnimSync::Insert(dir, x) => self.loose_insert.reset(dir, x),
        }
    }

    pub fn apply_send(&mut self, sync: AnimSync) {
        self.apply(sync.clone());
        if let Some(ref mut send) = self.net_queue {
            let message = Message::Anim(sync);
            send.lock().unwrap().push_back(message.into());
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AnimSync {
    Rotate(RotateDir),
    Insert(Direction, usize),
}

lazy_static! {
    pub static ref STATE: RwLock<AnimGlobalState> = { RwLock::new(AnimGlobalState::new()) };
}
