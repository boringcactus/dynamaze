use std::f64::consts::FRAC_PI_2;

const TARGET_STRIPE_LENGTH: f64 = 2.0;
const LOOSE_ROTATE_LENGTH: f64 = 0.3;

/// Tracks state of the target stripe animation
pub struct TargetStripeState {
    pub offset: f64,
}

impl TargetStripeState {
    fn new() -> TargetStripeState {
        TargetStripeState {
            offset: 0.0,
        }
    }

    fn advance_by(&mut self, ticks: f64) {
        self.offset = (self.offset + ticks) % TARGET_STRIPE_LENGTH;
    }

    pub fn pct_offset(&self) -> f64 {
        self.offset / TARGET_STRIPE_LENGTH
    }
}

/// Checks the direction in which the tile rotate animation spins
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
    fn new() -> LooseRotateState {
        LooseRotateState {
            angle: 0.0,
        }
    }

    pub fn reset(&mut self, dir: RotateDir) {
        self.angle += match dir {
            RotateDir::CW => -FRAC_PI_2,
            RotateDir::CCW => FRAC_PI_2,
        };
    }

    fn advance_by(&mut self, ticks: f64) {
        if self.angle == 0.0 {
            return;
        }
        let delta = FRAC_PI_2 / LOOSE_ROTATE_LENGTH;
        let (delta, clamp): (f64, fn(f64, f64) -> f64) = if self.angle.is_sign_positive() {
            (-delta, f64::max)
        } else {
            (delta, f64::min)
        };
        self.angle = clamp(self.angle + delta * ticks, 0.0);
    }
}

/// Tracks state of all currently running animations
pub struct AnimGlobalState {
    pub target_stripe: TargetStripeState,
    pub loose_rotate: LooseRotateState,
}

impl AnimGlobalState {
    pub fn new() -> AnimGlobalState {
        AnimGlobalState {
            target_stripe: TargetStripeState::new(),
            loose_rotate: LooseRotateState::new(),
        }
    }

    pub fn advance_by(&mut self, ticks: f64) {
        self.target_stripe.advance_by(ticks);
        self.loose_rotate.advance_by(ticks);
    }
}
