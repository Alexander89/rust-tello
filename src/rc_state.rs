use std::net::UdpSocket;
use crate::{ControllerState, Command};
use std::time::{SystemTime, Duration};

#[derive(Clone, Debug)]
pub struct RCState {
    left_right: f32,
    forward_back: f32,
    turn: f32,
    up_down: f32,

    max_speed: f32,
    start_engines: bool,
    start_engines_set_time: Option<SystemTime>,
}

impl RCState {
    pub fn new() -> RCState {
        RCState {
            left_right: 0.0,
            forward_back: 0.0,
            turn: 0.0,
            up_down: 0.0,
            max_speed: 1.0,
            start_engines: false,
            start_engines_set_time: None,
        }
    }

    pub fn start_engines(&mut self) {
        self.start_engines = true;
        self.start_engines_set_time = Some(SystemTime::now());
    }

    pub fn update_rc_state(&mut self, c_state: &ControllerState) {
        if c_state.a_down {
            self.go_left()
        } else if c_state.d_down {
            self.go_right()
        } else {
            self.lr_stop()
        }

        if c_state.w_down {
            self.go_forward()
        } else if c_state.s_down {
            self.go_back()
        } else {
            self.fb_stop()
        }

        if c_state.up_down {
            self.go_up()
        } else if c_state.down_down {
            self.go_down()
        } else {
            self.ud_stop()
        }

        if c_state.left_down {
            self.go_ccw()
        } else if c_state.right_down {
            self.go_cw()
        } else {
            self.turn_stop()
        }
    }

    pub fn send_command(&mut self, cmd: &Command) {
        if self.start_engines {
            cmd.send_stick(-1.0, -1.0, -1.0, 1.0, true).unwrap();

            if let Some(start) = self.start_engines_set_time {
                let elapsed = SystemTime::now().duration_since(start);
                if let Ok(time) = elapsed {
                    if time.as_millis() > 350 {
                        self.start_engines = false;
                    }
                } else {
                    self.start_engines = false;
                }
            } else {
                self.start_engines = false;
            }
        } else {
            cmd.send_stick(self.up_down, self.forward_back, self.left_right, self.turn, true).unwrap();
        }
    }
}

impl RCState {
    pub fn lr_stop(&mut self) {
        self.left_right = 0.0;
    }

    pub fn go_left(&mut self) {
        if self.left_right > 0.0 {
            self.lr_stop()
        } else {
            self.left_right = -1.0; // -= (self.max_speed + self.left_right) / 5.0;
        }
    }

    pub fn go_right(&mut self) {
        if self.left_right < 0.0 {
            self.lr_stop()
        } else {
            // += to go left
            self.left_right = 1.0; //+= (self.max_speed - self.left_right) / 5.0;
        }
    }
}

impl RCState {
    pub fn fb_stop(&mut self) {
        self.forward_back = 0.0;
    }

    pub fn go_back(&mut self) {
        if self.forward_back > 0.0 {
            self.fb_stop()
        } else {
            // -= to go back
            self.forward_back = -1.0; //-= (self.max_speed + self.forward_back) / 5.0;
        }
    }

    pub fn go_forward(&mut self) {
        if self.forward_back < 0.0 {
            self.fb_stop()
        } else {
            // += to go left
            self.forward_back = 1.0; //+= (self.max_speed - self.forward_back) / 5.0;
        }
    }
}

impl RCState {
    pub fn ud_stop(&mut self) {
        self.up_down = 0.0;
    }

    pub fn go_down(&mut self) {
        if self.up_down > 0.0 {
            self.ud_stop()
        } else {
            // -= to go down
            self.up_down = -1.0; //-= (self.max_speed + self.up_down) / 5.0;
        }
    }

    pub fn go_up(&mut self) {
        if self.up_down < 0.0 {
            self.ud_stop()
        } else {
            // += to go left
            self.up_down = 1.0; //+= (self.max_speed - self.up_down) / 5.0;
        }
    }
}

impl RCState {
    pub fn turn_stop(&mut self) {
        self.turn = 0.0;
    }

    pub fn go_ccw(&mut self) {
        if self.turn > 0.0 {
            self.turn_stop()
        } else {
            // -= to go ccw
            self.turn = -1.0; //-= (self.max_speed + self.turn) / 5.0;
        }
    }

    pub fn go_cw(&mut self) {
        if self.turn < 0.0 {
            self.turn_stop()
        } else {
            // += to go left
            self.turn = 1.0; //+= (self.max_speed - self.turn) / 5.0;
        }
    }
}

