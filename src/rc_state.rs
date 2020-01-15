use std::time::SystemTime;

/// represent the current input to remote control the drone.
///
#[derive(Clone, Debug)]
pub struct RCState {
    left_right: f32,
    forward_back: f32,
    turn: f32,
    up_down: f32,

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

    /// returns the current stick parameter to send them to the drone
    ///
    /// Actually, this is an workaround to keep the start_engines in this struct and
    /// don't move them to the Drone it self
    pub fn get_stick_parameter(&mut self) -> (f32, f32, f32, f32, bool) {
        if self.start_engines {
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
            (-1.0, -1.0, -1.0, 1.0, true)
        } else {
            (
                self.up_down,
                self.forward_back,
                self.left_right,
                self.turn,
                true,
            )
        }
    }
}

impl RCState {
    /// stop moving left or right by setting the axis to 0.0
    pub fn stop_left_right(&mut self) {
        self.left_right = 0.0;
    }

    /// set a fixed value of -1.0 to the left right axis to fly to the left
    pub fn go_left(&mut self) {
        if self.left_right > 0.0 {
            self.stop_left_right()
        } else {
            self.left_right = -1.0;
        }
    }

    /// set a fixed value of 1.0 to the left right axis to fly to the right
    pub fn go_right(&mut self) {
        if self.left_right < 0.0 {
            self.stop_left_right()
        } else {
            self.left_right = 1.0;
        }
    }

    /// set a analog value to the left right axis
    /// this value has to be between -1 and 1 (including), where -1 is left and 1 is right
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
            self.forward_back = -1.0;
        }
    }

    pub fn go_forward(&mut self) {
        if self.forward_back < 0.0 {
            self.stop_forward_backward()
        } else {
            self.forward_back = 1.0;
        }
    }

    /// set a analog value to the forward or back axis
    /// this value has to be between -1 and 1 (including), where -1 is back and 1 is forward
    pub fn go_forward_back(&mut self, value: f32) {
        assert!(value <= 1.0);
        assert!(value >= -1.0);

        self.forward_back = value;
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
            self.up_down = -1.0;
        }
    }

    pub fn go_up(&mut self) {
        if self.up_down < 0.0 {
            self.stop_up_down()
        } else {
            self.up_down = 1.0;
        }
    }

    /// set a analog value to the up or down axis
    /// this value has to be between -1 and 1 (including), where -1 is going down and 1 is flying up
    pub fn go_up_down(&mut self, value: f32) {
        assert!(value <= 1.0);
        assert!(value >= -1.0);

        self.up_down = value;
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
            self.turn = -1.0;
        }
    }

    pub fn go_cw(&mut self) {
        if self.turn < 0.0 {
            self.stop_turn()
        } else {
            self.turn = 1.0;
        }
    }

    /// Set a analog value to turn the drone
    /// This value has to be between -1 and 1 (including), where -1 is turning ccw and 1 is turning cw
    pub fn turn(&mut self, value: f32) {
        assert!(value <= 1.0);
        assert!(value >= -1.0);

        self.turn = value;
    }
}
