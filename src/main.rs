use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use std::time::Duration;
use std::string::String;
use std::path::Path;
mod meta_data;

mod crc;
mod command;
mod rc_state;
mod controller_state;
mod drone_messages;
mod drone_state;

use drone_state::DroneState;
use command::{Command, Flip, Message, CommandIds, PackageData, ResponseMsg};
use rc_state::RCState;
use controller_state::ControllerState;

// extern crate gstreamer as gst;
// use gst::prelude::*;

// extern crate glib;
#[derive(Debug)]
struct MissingElement(&'static str);


// fn update_rc_state(rc_state: RCState, c_state: &ControllerState) -> RCState {
//     let mut new_rc_state = rc_state.clone();

//     if c_state.a_down {
//         new_rc_state = new_rc_state.go_left()
//     } else if c_state.d_down {
//         new_rc_state = new_rc_state.go_right()
//     } else {
//         new_rc_state = new_rc_state.lr_stop()
//     }

//     if c_state.w_down {
//         new_rc_state = new_rc_state.go_forward()
//     } else if c_state.s_down {
//         new_rc_state = new_rc_state.go_back()
//     } else {
//         new_rc_state = new_rc_state.fb_stop()
//     }

//     if c_state.up_down {
//         new_rc_state = new_rc_state.go_up()
//     } else if c_state.down_down {
//         new_rc_state = new_rc_state.go_down()
//     } else {
//         new_rc_state = new_rc_state.ud_stop()
//     }

//     if c_state.left_down {
//         new_rc_state = new_rc_state.go_ccw()
//     } else if c_state.right_down {
//         new_rc_state = new_rc_state.go_cw()
//     } else {
//         new_rc_state = new_rc_state.turn_stop()
//     }

//     new_rc_state
// }
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;
// const VIDEO_WIDTH: u32 = 1280;
// const VIDEO_HEIGHT: u32 = 720;


fn main() -> Result<(), String> {
    let mut drone = Command::new("192.168.10.1:8889");
    let mut drone_state = DroneState::new();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem.window("TELLO drone", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .expect("could not initialize video subsystem");

    let mut canvas = window.into_canvas().build()
        .expect("could not make a canvas");

    let ttf_context = sdl2::ttf::init().expect("could not initialize font subsystem");

    let texture_creator = canvas.texture_creator();
    let font_path: &Path = Path::new("./DejaVuSans.ttf");
    let font = ttf_context.load_font(font_path, 24).expect("load font");
    let keys_target = Rect::new((WINDOW_WIDTH - 250) as i32, 0, 250, 200);
    let key_text = "i: connect\nk: take_off\nl: land/cancel\nv: start/stop video";

    let mut event_pump = sdl_context.event_pump()?;
    let mut i = 0;
    let mut land = false;
    let mut video_on = false;
    let mut bounce_on = false;
    let mut keyboard = ControllerState::new();
    let mut rc_state = RCState::new();
    let mut status_counter = 0;

    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();

        let surface = font.render(key_text).blended_wrapped(Color::RGB(0, 0, 0), 250).unwrap();
        let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
        canvas.copy(&texture, None, Some(keys_target))?;


        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                Event::KeyDown { keycode: Some(Keycode::I), .. } => {
                    drone.connect(11111);
                },
                Event::KeyDown { keycode: Some(Keycode::K), .. } => {
                    land = false;
                    drone.take_off().unwrap();
                },
                Event::KeyDown { keycode: Some(Keycode::O), .. } => {
                    drone.start_engines(&mut rc_state);
                },
                Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                  drone.throw_and_go().unwrap();
                },
                Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                    if land == false {
                        land = true;
                        drone.land().unwrap();
                    } else {
                        land = false;
                        drone.stop_land().unwrap();
                    }
                },
                Event::KeyDown { keycode: Some(Keycode::V), .. } => {
                    if video_on == false {
                        video_on = true;
                        drone.start_video().unwrap();
                    } else {
                        video_on = false;
                        // @TODO unknown command for stop_video
                        drone.start_video().unwrap();
                    }
                },
                Event::KeyDown { keycode: Some(Keycode::H), .. } => {
                    if bounce_on == false {
                        bounce_on = true;
                        drone.bounce().unwrap();
                    } else {
                        bounce_on = false;
                        drone.bounce_stop().unwrap();
                    }
                },

                Event::KeyDown { keycode: Some(Keycode::T), .. } => drone.flip(Flip::ForwardLeft).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::Z), .. } => drone.flip(Flip::Forward).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::U), .. } => drone.flip(Flip::ForwardRight).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::G), .. } => drone.flip(Flip::Left).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::J), .. } => drone.flip(Flip::Right).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::B), .. } => drone.flip(Flip::BackLeft).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::N), .. } => drone.flip(Flip::Back).unwrap(),
                Event::KeyDown { keycode: Some(Keycode::M), .. } => drone.flip(Flip::BackRight).unwrap(),

                Event::KeyDown { keycode: Some(keycode), .. } => keyboard.key_down(keycode),
                Event::KeyUp { keycode: Some(keycode), .. } => keyboard.key_up(keycode),
                _ => {}
            }
        }

        if let Some(msg) = drone.poll() {
            match msg {
                Message::Data(d) if d.cmd == CommandIds::FlightMsg => {
                    drone_state.update(&d.data);

                    if let PackageData::FlightData(d) = d.data {

                        println!("battery {}", d.battery_percentage);
                        status_counter += 1;
                        if status_counter == 3 {
                            drone.get_version().unwrap();
                            drone.get_video_bitrate(4).unwrap();
                            drone.get_alt_limit().unwrap();
                            drone.get_battery_threshold().unwrap();
                            drone.get_att_angle().unwrap();
                            drone.get_region().unwrap();
                            drone.set_exposure(2).unwrap();
                        }
                    }
                }
                Message::Data(d) if d.cmd == CommandIds::LogHeaderMsg => (),
                Message::Data(d) => {
                    drone_state.update(&d.data);
                    println!("msg {:?}", d.clone());
                }
                Message::Response(r) => {
                    match r {
                        ResponseMsg::Connected(_) => println!("connected"),
                        _ => ()
                    }
                }
            }
        }

        rc_state.update_rc_state(&keyboard);
        rc_state.send_command(&drone);

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }

    Ok(())
}
