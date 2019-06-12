const TARGET_STRIPE_LENGTH: f64 = 2.0;

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

/// Tracks state of all currently running animations
pub struct AnimGlobalState {
    pub target_stripe: TargetStripeState,
}

impl AnimGlobalState {
    pub fn new() -> AnimGlobalState {
        AnimGlobalState {
            target_stripe: TargetStripeState::new(),
        }
    }

    pub fn advance_by(&mut self, ticks: f64) {
        self.target_stripe.advance_by(ticks);
    }
}
