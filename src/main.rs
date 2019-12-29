use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use std::time::Duration;
use std::string::String;
use std::net::{SocketAddr, UdpSocket};
use std::convert::TryInto;
use std::path::Path;
use std::ops::Deref;

mod meta_data;
use meta_data::MetaData;

mod rc_state;
use rc_state::RCState;
mod controller_state;
use controller_state::ControllerState;

extern crate gstreamer as gst;
use gst::prelude::*;

extern crate glib;
#[derive(Debug)]
struct MissingElement(&'static str);

use std::io::{Cursor};

fn enable(socket: &UdpSocket) -> usize {
    println!("enable");
    socket.send(b"command").unwrap()
}
fn lift_off(socket: &UdpSocket) -> usize {
    println!("take off");
    socket.send(b"takeoff").unwrap()
}
fn land(socket: &UdpSocket) -> usize {
    println!("land");
    socket.send(b"land").unwrap()
}
fn video_on(socket: &UdpSocket) -> usize {
    println!("video on");
    socket.send(b"streamon").unwrap()
}
fn video_off(socket: &UdpSocket) -> usize {
    println!("video off");
    socket.send(b"streamoff").unwrap()
}
// fn left(socket: &UdpSocket, step: u32) -> usize {
//     println!("left");
//     let command = format!("left {}", step);
//     socket.send(&command.into_bytes()).unwrap()
// }
// fn right(socket: &UdpSocket, step: u32) -> usize {
//     println!("right");
//     let command = format!("right {}", step);
//     socket.send(&command.into_bytes()).unwrap()
// }
// fn forward(socket: &UdpSocket, step: u32) -> usize {
//     println!("forward");
//     let command = format!("forward {}", step);
//     socket.send(&command.into_bytes()).unwrap()Encoder
// }
// fn back(socket: &UdpSocket, step: u32) -> usize {
//     println!("back");
//     let command = format!("back {}", step);
//     socket.send(&command.into_bytes()).unwrap()
// }
// fn cw(socket: &UdpSocket, step: u32) -> usize {
//     println!("cw");
//     let command = format!("cw {}", step);
//     socket.send(&command.into_bytes()).unwrap()
// }
// fn ccw(socket: &UdpSocket, step: u32) -> usize {
//     println!("ccw");
//     let command = format!("ccw {}", step);
//     socket.send(&command.into_bytes()).unwrap()
// }

fn update_rc_state(rc_state: RCState, c_state: &ControllerState) -> RCState {
    let mut new_rc_state = rc_state.clone();
    
    if c_state.a_down {
        new_rc_state = new_rc_state.go_left()
    } else if c_state.d_down {
        new_rc_state = new_rc_state.go_right()
    } else {
        new_rc_state = new_rc_state.lr_stop()
    }

    if c_state.w_down {
        new_rc_state = new_rc_state.go_forward()
    } else if c_state.s_down {
        new_rc_state = new_rc_state.go_back()
    } else {
        new_rc_state = new_rc_state.fb_stop()
    }

    if c_state.up_down {
        new_rc_state = new_rc_state.go_up()
    } else if c_state.down_down {
        new_rc_state = new_rc_state.go_down()
    } else {
        new_rc_state = new_rc_state.ud_stop()
    }

    if c_state.left_down {
        new_rc_state = new_rc_state.go_ccw()
    } else if c_state.right_down {
        new_rc_state = new_rc_state.go_cw()
    } else {
        new_rc_state = new_rc_state.turn_stop()
    }

    new_rc_state
}
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;
const VIDEO_WIDTH: u32 = 1280;
const VIDEO_HEIGHT: u32 = 720;


fn main() -> Result<(), String> {
    let command_bind_addr = SocketAddr::from(([0, 0, 0, 0], 8889));
    let command_socket = UdpSocket::bind(&command_bind_addr).expect("couldn't bind to command address");
    command_socket.connect("192.168.10.1:8889").expect("connect command socket failed");

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

    Ok(())
}