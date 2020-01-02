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
    let drone = Command::new("192.168.10.1:8889");
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
                            drone.get_video_bitrate().unwrap();
                            drone.get_alt_limit().unwrap();
                            drone.get_battery_threshold().unwrap();
                            drone.get_att_angle().unwrap();
                            drone.get_region().unwrap();
                            drone.set_exposure().unwrap();
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
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 10));
    }
    /*    
    let video_bind_addr = SocketAddr::from(([0, 0, 0, 0], 11112));
    let video_socket = UdpSocket::bind(&video_bind_addr).expect("couldn't bind to stream address");
    video_socket.set_nonblocking(true).unwrap();

    let meta_bind_addr = SocketAddr::from(([0, 0, 0, 0], 8890));
    let meta_socket = UdpSocket::bind(&meta_bind_addr).expect("couldn't bind to meta address");
    meta_socket.set_nonblocking(true).unwrap();

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
    let mut font = ttf_context.load_font(font_path, 128).expect("load font");
    font.set_style(sdl2::ttf::FontStyle::BOLD);
    let meta_target = Rect::new(0, (WINDOW_HEIGHT - 50) as i32, WINDOW_WIDTH, 40);
    let info_target = Rect::new((WINDOW_WIDTH / 100 * 20) as i32, (WINDOW_HEIGHT / 2 - 50) as i32, WINDOW_WIDTH / 100 * 60, 100);

    gst::init().unwrap();
    //gst-launch-1.0 shmsrc socket-path=/tmp/blah ! "video/x-raw, format=YUY2, color-matrix=sdtv, chroma-site=mpeg2, width=(int)320, height=(int)240, framerate=(fraction)30/1" ! queue ! videoconvert ! autovideosink
    let _pipeline = gst::Pipeline::new(None);
    let _src = gst::ElementFactory::make("shmsrc", None).map_err(|_| MissingElement("shmsrc")).unwrap();
    let _decodebin = gst::ElementFactory::make("decodebin", None).map_err(|_| MissingElement("decodebin")).unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;
    let mut i = 0;
    let mut drone = RCState::new();
    let mut keyboard = ControllerState::new();

    let mut video_buf = [0; 65_535];
    let mut meta_buf = [0; 1440];
    let mut info_text = String::from("Not Connected or Enabled. Activate drone with 'i'");
    let mut drone_state = String::from("");

    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                Event::KeyDown { keycode: Some(Keycode::I), .. } => {
                    enable(&command_socket);
                },
                Event::KeyDown { keycode: Some(Keycode::K), .. } => {
                    lift_off(&command_socket);
                },
                Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                    land(&command_socket);
                },
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                    video_on(&command_socket);
                },
                Event::KeyDown { keycode: Some(Keycode::Delete), .. } => {
                    video_off(&command_socket);
                },
                Event::KeyDown { keycode, .. } => {
                    if let Some(keycode) = keycode {
                        keyboard = keyboard.key_down(keycode);
                    }
                },
                Event::KeyUp { keycode, .. } => {
                    if let Some(keycode) = keycode {
                        keyboard = keyboard.key_up(keycode);
                    }
                },
                _ => {}
            }
        }

        drone = update_rc_state(drone, &keyboard);
        drone.publish(&command_socket);

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        
        // render a surface, and convert it to a texture bound to the canvas
        if drone_state.len() != 0 {
            let surface = font.render(drone_state.deref()).blended(Color::RGB(0, 0, 0)).unwrap();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            canvas.copy(&texture, None, Some(meta_target))?;
        }

        if info_text.len() != 0 {
            canvas.set_draw_color(Color::RGB(0, 255, 0));
            let surface = font.render(info_text.deref()).blended(Color::RGB(0, 0, 0)).unwrap();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            canvas.copy(&texture, None, Some(info_target))?;
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));

        let mut udp_package = [0; 1461];
        let x_ptr = video_buf.as_mut_ptr();
        let mut walker: usize = 0;
        'reedVideoData: loop {
            match video_socket.peek(&mut udp_package) {
                Ok(received) => {
                    if walker != 0 && udp_package[0..5] == [0,0,0,1,65] {
                        break 'reedVideoData;
                    }

                    video_socket.recv(&mut udp_package).unwrap();
                    unsafe {
                        for i in 0..udp_package.len() {
                            (*x_ptr.add(walker + i)) = udp_package[i];
                        }
                    }
                    walker += received;

                },
                Err(_e) => break 'reedVideoData,
            }
        }
        if walker > 0 {
            let vec_data = video_buf.to_vec();
            let mut _cursor = Cursor::new(&vec_data);
            // match mp4::read_mp4(&mut cursor, &mut context) {
            //     Ok(_) => {
            //         println!("received stream size {} length {:?}", walker, context.timescale.unwrap());
            //         println!("parse video stream")
            //     },
            //     Err(e) => println!("parse video failed {:?}", e),
            // };

            // vec_data -> bild

            // let surface = font.render(info_text.deref()).blended(Color::RGB(0, 0, 0)).unwrap();
            // let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            // canvas.copy(&texture, None, Some(info_target))?;
            
        }

        if let Ok(received) = meta_socket.recv(&mut meta_buf) {
            info_text = String::from("");
            let meta_data_str = String::from_utf8(meta_buf[..received].to_vec()).unwrap();
            let data: MetaData = meta_data_str.try_into().unwrap();
            drone_state = format!("{:?}", data);
        }
    }

    */
    Ok(())
}