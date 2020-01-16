use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::path::Path;
use std::string::String;
use std::time::Duration;

use std::ops::Deref;
use tello::{Drone, Flip, Message, Package, PackageData, RCState, ResponseMsg};

// extern crate glib;
#[derive(Debug)]
struct MissingElement(&'static str);

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("TELLO drone", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .expect("could not initialize video subsystem");

    let mut canvas = window
        .into_canvas()
        .build()
        .expect("could not make a canvas");

    let ttf_context = sdl2::ttf::init().expect("could not initialize font subsystem");

    let texture_creator = canvas.texture_creator();
    let font_path: &Path = Path::new("./examples/DejaVuSans.ttf");
    let font = ttf_context.load_font(font_path, 24).expect("load font");
    let keys_target = Rect::new((WINDOW_WIDTH - 250) as i32, 0, 250, 196);
    let key_texture = texture_creator.create_texture_from_surface(
        &font
            .render("i: connect\nk: take off\no: manual take off\np: throw 'n go\nl: land/cancel\nv: start video\nESC: Exit")
            .blended_wrapped(Color::RGB(0, 0, 0), 250)
            .unwrap()
    ).unwrap();
    let control_target = Rect::new(10, 0, 240, 112);
    let control_texture = texture_creator
        .create_texture_from_surface(
            &font
                .render("w/s: forward/back\na/d: left/right\nup/down: up/down\nleft/right: turn")
                .blended_wrapped(Color::RGB(0, 0, 0), 240)
                .unwrap(),
        )
        .unwrap();
    let stats_target = Rect::new(50, WINDOW_HEIGHT as i32 - 40, WINDOW_WIDTH - 100, 40);

    let mut event_pump = sdl_context.event_pump()?;

    let mut land = false;
    let mut video_on = false;
    let mut bounce_on = false;
    let mut keyboard = ControllerState::default();

    'running: loop {
        canvas.set_draw_color(Color::RGB(80, 64, 255 - 80));
        canvas.clear();
        canvas.copy(&key_texture, None, Some(keys_target))?;
        canvas.copy(&control_texture, None, Some(control_target))?;

        if let Some(data) = drone.drone_meta.get_flight_data() {
            let d = format!("{:?}", data);
            let surface_stats = font.render(d.deref()).blended(Color::RGB(0, 0, 0)).unwrap();
            let texture_stats = texture_creator
                .create_texture_from_surface(&surface_stats)
                .unwrap();
            canvas.copy(&texture_stats, None, Some(stats_target))?;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::I),
                    ..
                } => {
                    drone.connect(11111);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::K),
                    ..
                } => {
                    land = false;
                    drone.take_off().unwrap();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::O),
                    ..
                } => {
                    drone.rc_state.start_engines();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    drone.throw_and_go().unwrap();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    ..
                } => {
                    if land == false {
                        land = true;
                        drone.land().unwrap();
                    } else {
                        land = false;
                        drone.stop_land().unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::V),
                    ..
                } => {
                    if video_on == false {
                        video_on = true;
                        drone.start_video().unwrap();
                    } else {
                        video_on = false;
                        // @TODO unknown command for stop_video
                        drone.start_video().unwrap();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::H),
                    ..
                } => {
                    if bounce_on == false {
                        bounce_on = true;
                        drone.bounce().unwrap();
                    } else {
                        bounce_on = false;
                        drone.bounce_stop().unwrap();
                    }
                }

                Event::KeyDown {
                    keycode: Some(Keycode::T),
                    ..
                } => drone.flip(Flip::ForwardLeft).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    ..
                } => drone.flip(Flip::Forward).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::U),
                    ..
                } => drone.flip(Flip::ForwardRight).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::G),
                    ..
                } => drone.flip(Flip::Left).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::J),
                    ..
                } => drone.flip(Flip::Right).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::B),
                    ..
                } => drone.flip(Flip::BackLeft).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    ..
                } => drone.flip(Flip::Back).unwrap(),
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    ..
                } => drone.flip(Flip::BackRight).unwrap(),

                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => keyboard.handle_key_down(keycode),
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => keyboard.handle_key_up(keycode),
                _ => {}
            }
        }

        keyboard.update_rc_state(&mut drone.rc_state);

        if let Some(msg) = drone.poll() {
            match msg {
                Message::Data(Package {data: PackageData::FlightData(d), ..}) => {
                    println!("battery {}", d.battery_percentage);
                }
                Message::Data(d) /*if d.cmd != CommandIds::LogHeaderMsg*/ => {
                    println!("msg {:?}", d.clone());
                }
                Message::Response(ResponseMsg::Connected(_)) => {
                    println!("connected");
                }
                _ => ()
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }

    Ok(())
}

// represent the keyboard state.
// witch key is currently pressed
#[derive(Clone, Debug, Default)]
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
    /// handle the SDL key down event
    pub fn handle_key_down(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::A => {
                self.d_down = false;
                self.a_down = true;
            }
            Keycode::D => {
                self.a_down = false;
                self.d_down = true;
            }
            Keycode::W => {
                self.s_down = false;
                self.w_down = true;
            }
            Keycode::S => {
                self.w_down = false;
                self.s_down = true;
            }
            Keycode::Up => {
                self.down_down = false;
                self.up_down = true;
            }
            Keycode::Down => {
                self.up_down = false;
                self.down_down = true;
            }
            Keycode::Left => {
                self.right_down = false;
                self.left_down = true;
            }
            Keycode::Right => {
                self.left_down = false;
                self.right_down = true;
            }
            _ => (),
        }
    }
    /// handle the SDL key up event
    pub fn handle_key_up(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::A => self.a_down = false,
            Keycode::D => self.d_down = false,
            Keycode::W => self.w_down = false,
            Keycode::S => self.s_down = false,
            Keycode::Up => self.up_down = false,
            Keycode::Down => self.down_down = false,
            Keycode::Left => self.left_down = false,
            Keycode::Right => self.right_down = false,
            _ => (),
        }
    }

    pub fn update_rc_state(&self, rc_state: &mut RCState) {
        if self.a_down {
            rc_state.go_left()
        } else if self.d_down {
            rc_state.go_right()
        } else {
            rc_state.stop_left_right()
        }

        if self.w_down {
            rc_state.go_forward()
        } else if self.s_down {
            rc_state.go_back()
        } else {
            rc_state.stop_forward_back()
        }

        if self.up_down {
            rc_state.go_up()
        } else if self.down_down {
            rc_state.go_down()
        } else {
            rc_state.stop_up_down()
        }

        if self.left_down {
            rc_state.go_ccw()
        } else if self.right_down {
            rc_state.go_cw()
        } else {
            rc_state.stop_turn()
        }
    }
}
