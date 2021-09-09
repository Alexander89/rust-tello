#[derive(Default, Debug, PartialEq, Clone)]
pub struct Odometry {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rot: f64,
}

impl Odometry {
    fn translate(&mut self, x: f64, y: f64) -> () {
        self.x += x * self.rot.cos() - y * self.rot.sin();
        self.y += x * self.rot.sin() + y * self.rot.cos();
    }

    pub fn reset(&mut self) -> () {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
        self.rot = 0.0;
    }

    pub fn up(&mut self, z: u32) -> () {
        self.z += z.max(20).min(500) as f64;
    }
    pub fn down(&mut self, z: u32) -> () {
        self.z -= z.max(20).min(500) as f64;
    }
    pub fn right(&mut self, x: u32) -> () {
        let x = x.max(20).min(500) as f64;
        self.translate(x as f64, 0.0);
    }
    pub fn left(&mut self, x: u32) -> () {
        let x = x.max(20).min(500) as f64;
        self.translate(-x, 0.0);
    }
    pub fn forward(&mut self, y: u32) -> () {
        let y = y.max(20).min(500) as f64;
        self.translate(0.0, y);
    }
    pub fn back(&mut self, y: u32) -> () {
        let y = y.max(20).min(500) as f64;
        self.translate(0.0, -y);
    }
    pub fn cw(&mut self, rot: u32) -> () {
        let mut rot: f64 = rot.max(1).min(3600).into();
        rot = rot / 180.0 * std::f64::consts::PI;
        self.rot -= rot
    }
    pub fn ccw(&mut self, rot: u32) -> () {
        let mut rot: f64 = rot.max(1).min(3600).into();
        rot = rot / 180.0 * std::f64::consts::PI;
        self.rot += rot
    }
}

#[test]
pub fn test_go_back_again() {
    let mut p = Odometry::default();
    p.forward(100);
    p.cw(45);
    p.forward(100);
    p.cw(180);
    p.forward(100);
    p.ccw(45);
    p.forward(100);
    let rx = p.x.round();
    let ry = p.y.round();
    assert_eq!(rx, 0.0f64);
    assert_eq!(ry, 0.0f64);
}
#[test]
pub fn test_go_right() {
    let mut p = Odometry::default();
    p.forward(100);
    assert_eq!(p.x, 0.0f64);
    assert_eq!(p.y, 100.0f64);
    p.cw(90);
    p.forward(100);
    assert_eq!(p.x, 100.0f64);
    assert_eq!(p.y, 100.0f64);
}
#[test]
pub fn test_go_left() {
    let mut p = Odometry::default();
    p.forward(100);
    assert_eq!(p.x, 0.0f64);
    assert_eq!(p.y, 100.0f64);
    p.ccw(90);
    p.forward(100);
    assert_eq!(p.x, -100.0f64);
    assert_eq!(p.y, 100.0f64);
}
#[test]
pub fn test_go_square() {
    let mut p = Odometry::default();
    p.forward(100);
    assert_eq!(p.x, 0.0f64);
    assert_eq!(p.y, 100.0f64);
    p.cw(90);
    p.forward(100);
    assert_eq!(p.x.round(), 100.0f64);
    assert_eq!(p.y.round(), 100.0f64);
    p.cw(90);
    p.forward(200);
    assert_eq!(p.x.round(), 100.0f64);
    assert_eq!(p.y.round(), -100.0f64);
    p.cw(90);
    p.forward(200);
    assert_eq!(p.x.round(), -100.0f64);
    assert_eq!(p.y.round(), -100.0f64);
}
