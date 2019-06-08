use gilrs::{Event as GilEvent, Gilrs};
use gilrs::ev::{
    Axis as GilAxis,
    Button as GilButton,
    EventType as GilEventType,
};
use piston::input::{
    Button as PistonButton,
    ButtonArgs as PistonButtonArgs,
    ButtonState as PistonButtonState,
    Event as PistonEvent,
    Input as PistonInput,
    Key,
};

enum Axis {
    X,
    Y,
}

trait KnowsAxis {
    fn axis(&self) -> Option<Axis>;
}

impl KnowsAxis for GilAxis {
    fn axis(&self) -> Option<Axis> {
        use GilAxis::*;
        Some(match *self {
            LeftStickX | RightStickX | DPadX => Axis::X,
            LeftStickY | RightStickY | DPadY => Axis::Y,
            LeftZ | RightZ | Unknown => {
                return None;
            }
        })
    }
}

pub struct Handler {
    gilrs: Gilrs,
}

fn map_button(button: GilButton) -> Option<Key> {
    use GilButton::*;
    Some(match button {
        LeftTrigger | LeftTrigger2 => Key::LShift,
        RightTrigger | RightTrigger2 => Key::RShift,
        North | East | South | West | C | Z => Key::Space,
        DPadUp => Key::Up,
        DPadDown => Key::Down,
        DPadLeft => Key::Left,
        DPadRight => Key::Right,
        Select | Start | Mode | LeftThumb | RightThumb | Unknown => {
            return None;
        }
    })
}

impl Handler {
    pub fn new() -> Self {
        Handler {
            gilrs: Gilrs::new().unwrap(),
        }
    }

    pub fn next_event(&mut self) -> Option<PistonEvent> {
        self.gilrs.next_event().and_then(|e| self.map(e)).map(|x| dbg!(x))
    }

    fn map(&mut self, event: GilEvent) -> Option<PistonEvent> {
        let (state, key) = self.map_event(event.event)?;
        let args = PistonButtonArgs {
            state,
            button: PistonButton::Keyboard(key),
            scancode: None,
        };

        Some(PistonEvent::Input(PistonInput::Button(args)))
    }

    fn map_event(&mut self, event: GilEventType) -> Option<(PistonButtonState, Key)> {
        use GilEventType::*;
        Some(match event {
            ButtonPressed(button, _) | ButtonRepeated(button, _) => (PistonButtonState::Press, map_button(button)?),
            ButtonReleased(button, _) => (PistonButtonState::Release, map_button(button)?),
            ButtonChanged(_, _, _) | AxisChanged(_, _, _) | Connected | Disconnected | Dropped => {
                return None;
            }
        })
    }
}
