use super::Drone;
use std::time::SystemTime;

/// represent the current input to remote control the drone.
///
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

impl Default for RCState {
    fn default() -> Self {
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
}

impl RCState {
    /// set the rc-controller to the mode to hold down the key-combination to do an manual take_off.
    ///
    pub fn start_engines(&mut self) {
        self.start_engines = true;
        self.start_engines_set_time = Some(SystemTime::now());
    }

    pub fn send_command(&mut self, cmd: &Drone) {
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
            cmd.send_stick(
                self.up_down,
                self.forward_back,
                self.left_right,
                self.turn,
                true,
            )
            .unwrap();
        }
    }
}

impl RCState {
    pub fn stop_left_right(&mut self) {
        self.left_right = 0.0;
    }

    pub fn go_left(&mut self) {
        if self.left_right > 0.0 {
            self.stop_left_right()
        } else {
            self.left_right = -1.0; // -= (self.max_speed + self.left_right) / 5.0;
        }
    }

    pub fn go_right(&mut self) {
        if self.left_right < 0.0 {
            self.stop_left_right()
        } else {
            // += to go left
            self.left_right = 1.0; //+= (self.max_speed - self.left_right) / 5.0;
        }
    }

    ///
    pub fn go_left_right(&mut self, value: f32) {
        assert!(value <= 1.0);
        assert!(value >= -1.0);

        self.left_right = value;
    }
}

impl RCState {
    pub fn stop_forward_backward(&mut self) {
        self.forward_back = 0.0;
    }

    pub fn go_back(&mut self) {
        if self.forward_back > 0.0 {
            self.stop_forward_backward()
        } else {
            // -= to go back
            self.forward_back = -1.0; //-= (self.max_speed + self.forward_back) / 5.0;
        }
    }

    pub fn go_forward(&mut self) {
        if self.forward_back < 0.0 {
            self.stop_forward_backward()
        } else {
            // += to go left
            self.forward_back = 1.0; //+= (self.max_speed - self.forward_back) / 5.0;
        }
    }
}

impl RCState {
    pub fn stop_up_down(&mut self) {
        self.up_down = 0.0;
    }

    pub fn go_down(&mut self) {
        if self.up_down > 0.0 {
            self.stop_up_down()
        } else {
            // -= to go down
            self.up_down = -1.0; //-= (self.max_speed + self.up_down) / 5.0;
        }
    }

    pub fn go_up(&mut self) {
        if self.up_down < 0.0 {
            self.stop_up_down()
        } else {
            // += to go left
            self.up_down = 1.0; //+= (self.max_speed - self.up_down) / 5.0;
        }
    }
}

impl RCState {
    pub fn stop_turn(&mut self) {
        self.turn = 0.0;
    }

    pub fn go_ccw(&mut self) {
        if self.turn > 0.0 {
            self.stop_turn()
        } else {
            // -= to go ccw
            self.turn = -1.0; //-= (self.max_speed + self.turn) / 5.0;
        }
    }

    pub fn go_cw(&mut self) {
        if self.turn < 0.0 {
            self.stop_turn()
        } else {
            // += to go left
            self.turn = 1.0; //+= (self.max_speed - self.turn) / 5.0;
        }
    }
}
