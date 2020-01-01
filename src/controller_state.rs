use sdl2::keyboard::Keycode;

#[derive(Clone, Debug)]
pub struct ControllerState {
    pub a_down: bool,
    pub d_down: bool,
    pub w_down: bool,
    pub s_down: bool,
    pub up_down: bool,
    pub down_down: bool,
    pub left_down: bool,
    pub right_down: bool,
}

impl ControllerState {
    pub fn new() -> ControllerState {
        ControllerState {
            a_down: false,
            d_down: false,
            w_down: false,
            s_down: false,
            up_down: false,
            down_down: false,
            left_down: false,
            right_down: false,
        }
    }

    pub fn key_down(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::A => {
                self.d_down = false;
                self.a_down = true;
            },
            Keycode::D => {
                self.a_down = false;
                self.d_down = true;
            },
            Keycode::W => {
                self.s_down = false;
                self.w_down = true;
            },
            Keycode::S => {
                self.w_down = false;
                self.s_down = true;
            },
            Keycode::Up => {
                self.down_down = false;
                self.up_down = true;
            },
            Keycode::Down => {
                self.up_down = false;
                self.down_down = true;
            },
            Keycode::Left => {
                self.right_down = false;
                self.left_down = true;
            },
            Keycode::Right => {
                self.left_down = false;
                self.right_down = true;
            },
            _ => ()
        }
    }
    pub fn key_up(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::A => self.a_down = false,
            Keycode::D => self.d_down = false,
            Keycode::W => self.w_down = false,
            Keycode::S => self.s_down = false,
            Keycode::Up => self.up_down = false,
            Keycode::Down => self.down_down = false,
            Keycode::Left => self.left_down = false,
            Keycode::Right => self.right_down = false,
            _ => ()
        }
    }
}