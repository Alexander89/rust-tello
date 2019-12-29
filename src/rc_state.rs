use std::net::UdpSocket;

#[derive(Clone, Debug)]
pub struct RCState {
    left_right: f32,
    forward_back: f32,
    turn: f32,
    up_down: f32,

    max_speed: f32,
}

impl RCState {
    pub fn new() -> RCState {
        RCState {
            left_right: 0.0,
            forward_back: 0.0,
            turn: 0.0,
            up_down: 0.0,
            max_speed: 100.0,
        }
    }

    pub fn publish(self: &RCState, socket: &UdpSocket) {
        let command = format!("rc {:.0} {:.0} {:.0} {:.0}", self.left_right, self.forward_back, self.up_down, self.turn);
        //println!("command {}", command);
        socket.send(&command.into_bytes()).unwrap();
    }
}

impl RCState {
    pub fn lr_stop(self: &RCState) -> RCState {
        let mut new = self.clone();
        new.left_right = 0.0;
        new
    }

    pub fn go_left(self: &RCState) -> RCState {
        if self.left_right > 0.0 {
            self.lr_stop()
        } else {
            let mut go_left = self.clone();
            // -= to go left
            go_left.left_right  -= (go_left.max_speed + go_left.left_right) / 5.0;
            go_left
        }
    }

    pub fn go_right(self: &RCState) -> RCState {
        if self.left_right < 0.0 {
            self.lr_stop()
        } else {
            let mut go_right = self.clone();
            // += to go left
            go_right.left_right += (go_right.max_speed - go_right.left_right) / 5.0;
            go_right
        }
    }
}

impl RCState {
    pub fn fb_stop(self: &RCState) -> RCState {
        let mut new = self.clone();
        new.forward_back = 0.0;
        new
    }

    pub fn go_back(self: &RCState) -> RCState {
        if self.forward_back > 0.0 {
            self.fb_stop()
        } else {
            let mut go_back = self.clone();
            // -= to go back
            go_back.forward_back  -= (go_back.max_speed + go_back.forward_back) / 5.0;
            go_back
        }
    }

    pub fn go_forward(self: &RCState) -> RCState {
        if self.forward_back < 0.0 {
            self.fb_stop()
        } else {
            let mut go_forward = self.clone();
            // += to go left
            go_forward.forward_back += (go_forward.max_speed - go_forward.forward_back) / 5.0;
            go_forward
        }
    }
}

impl RCState {
    pub fn ud_stop(self: &RCState) -> RCState {
        let mut new = self.clone();
        new.up_down = 0.0;
        new
    }

    pub fn go_down(self: &RCState) -> RCState {
        if self.up_down > 0.0 {
            self.ud_stop()
        } else {
            let mut go_down = self.clone();
            // -= to go down
            go_down.up_down  -= (go_down.max_speed + go_down.up_down) / 5.0;
            go_down
        }
    }

    pub fn go_up(self: &RCState) -> RCState {
        if self.up_down < 0.0 {
            self.ud_stop()
        } else {
            let mut go_up = self.clone();
            // += to go left
            go_up.up_down += (go_up.max_speed - go_up.up_down) / 5.0;
            go_up
        }
    }
}

impl RCState {
    pub fn turn_stop(self: &RCState) -> RCState {
        let mut new = self.clone();
        new.turn = 0.0;
        new
    }

    pub fn go_ccw(self: &RCState) -> RCState {
        if self.turn > 0.0 {
            self.turn_stop()
        } else {
            let mut go_ccw = self.clone();
            // -= to go ccw
            go_ccw.turn -= (go_ccw.max_speed + go_ccw.turn) / 5.0;
            go_ccw
        }
    }

    pub fn go_cw(self: &RCState) -> RCState {
        if self.turn < 0.0 {
            self.turn_stop()
        } else {
            let mut go_cw = self.clone();
            // += to go left
            go_cw.turn += (go_cw.max_speed - go_cw.turn) / 5.0;
            go_cw
        }
    }
}

