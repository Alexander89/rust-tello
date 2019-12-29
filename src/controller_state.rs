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

    pub fn key_down(self: &ControllerState, keycode: Keycode) -> ControllerState {
        match keycode {
            Keycode::A => {
                let mut n_cs = self.clone();
                n_cs.d_down = false;
                n_cs.a_down = true;
                return n_cs
            }
            Keycode::D => {
                let mut n_cs = self.clone();
                n_cs.a_down = false;
                n_cs.d_down = true;
                return n_cs
            }
            Keycode::W => {
                let mut n_cs = self.clone();
                n_cs.s_down = false;
                n_cs.w_down = true;
                return n_cs
            }
            Keycode::S => {
                let mut n_cs = self.clone();
                n_cs.w_down = false;
                n_cs.s_down = true;
                return n_cs
            }
            Keycode::Up => {
                let mut n_cs = self.clone();
                n_cs.down_down = false;
                n_cs.up_down = true;
                return n_cs
            }
            Keycode::Down => {
                let mut n_cs = self.clone();
                n_cs.up_down = false;
                n_cs.down_down = true;
                return n_cs
            }
            Keycode::Left => {
                let mut n_cs = self.clone();
                n_cs.right_down = false;
                n_cs.left_down = true;
                return n_cs
            }
            Keycode::Right => {
                let mut n_cs = self.clone();
                n_cs.left_down = false;
                n_cs.right_down = true;
                return n_cs
            }
            _ => self.clone()
        }
    }
    pub fn key_up(self: &ControllerState, keycode: Keycode) -> ControllerState {
        match keycode {
            Keycode::A => {
                let mut n_cs = self.clone();
                n_cs.a_down = false;
                return n_cs
            }
            Keycode::D => {
                let mut n_cs = self.clone();
                n_cs.d_down = false;
                return n_cs
            }
            Keycode::W => {
                let mut n_cs = self.clone();
                n_cs.w_down = false;
                return n_cs
            }
            Keycode::S => {
                let mut n_cs = self.clone();
                n_cs.s_down = false;
                return n_cs
            }
            Keycode::Up => {
                let mut n_cs = self.clone();
                n_cs.up_down = false;
                return n_cs
            }
            Keycode::Down => {
                let mut n_cs = self.clone();
                n_cs.down_down = false;
                return n_cs
            }
            Keycode::Left => {
                let mut n_cs = self.clone();
                n_cs.left_down = false;
                return n_cs
            }
            Keycode::Right => {
                let mut n_cs = self.clone();
                n_cs.right_down = false;
                return n_cs
            }
            _ => self.clone()
        }
    }
}