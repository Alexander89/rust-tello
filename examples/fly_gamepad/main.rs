use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::path::Path;
use std::string::String;
use std::time::Duration;
use std::ops::Deref;
use gilrs::{Gilrs, Axis, Event, Button, EventType};
use std::net::UdpSocket;

use tello::{Drone, Flip, Message, Package, PackageData, ResponseMsg};

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

fn main() -> Result<(), String> {

    let mut gilrs = Gilrs::new().map_err(|_| "gamepad not valid")?;

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

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
    let stats_target = Rect::new(50, WINDOW_HEIGHT as i32 - 40, WINDOW_WIDTH - 100, 40);

    let mut land = false;
    let mut bounce_on = false;

    let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldn't bind to address");
    'running: loop {
        canvas.set_draw_color(Color::RGB(80, 64, 255 - 80));
        canvas.clear();
        canvas.copy(&key_texture, None, Some(keys_target))?;

        // render drone state to the screen
        if let Some(data) = drone.drone_meta.get_flight_data() {
            let d = format!("{:?}", data);
            let surface_stats = font.render(d.deref()).blended(Color::RGB(0, 0, 0)).unwrap();
            let texture_stats = texture_creator
                .create_texture_from_surface(&surface_stats)
                .unwrap();
            canvas.copy(&texture_stats, None, Some(stats_target))?;
        }

        // map GamePad events to drone
        while let Some(Event { event, .. }) = gilrs.next_event() {
            match event {
                EventType::ButtonReleased(Button::Mode, _) => {
                    break 'running;
                }
                EventType::ButtonPressed(Button::Start, _) => {
                    drone.connect(11111);
                }
                EventType::ButtonPressed(Button::North, _) => {
                    if bounce_on == false {
                        bounce_on = true;
                        drone.bounce().map_err(|_| "bounce failed")?;
                    } else {
                        bounce_on = false;
                        drone.bounce_stop().map_err(|_| "bounce_stop failed")?;
                    }
                }
                EventType::ButtonPressed(Button::West, _) => {
                    drone.start_video().map_err(|_| "start_video failed")?;
                }
                EventType::ButtonPressed(Button::East, _) => {
                    drone.throw_and_go().map_err(|_| "throw_and_go failed")?;
                }
                EventType::ButtonPressed(Button::South, _) => {
                    drone.rc_state.start_engines();
                }
                EventType::ButtonPressed(Button::LeftTrigger, _) => {
                    land = false;
                    drone.take_off().map_err(|_| "take_off failed")?;
                }
                EventType::ButtonPressed(Button::RightTrigger, _) => {
                    if land {
                        land = false;
                        drone.stop_land().map_err(|_| "stop_land failed")?;
                    } else {
                        land = true;
                        drone.land().map_err(|_| "land failed")?;
                    }
                }
                // EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                //     drone.rc_state.go_left_right(value)
                // }
                // EventType::AxisChanged(Axis::LeftStickY, value, _) => {
                //     drone.rc_state.go_forward_back(value)
                // }
                // EventType::AxisChanged(Axis::RightStickX, value, _) => {
                //     drone.rc_state.turn(value)
                // }
                // EventType::AxisChanged(Axis::RightStickY, value, _) => {
                //     drone.rc_state.go_up_down(value)
                // }
                EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                    drone.rc_state.turn(value)
                }
                EventType::AxisChanged(Axis::LeftStickY, value, _) => {
                    drone.rc_state.go_up_down(value)
                }
                EventType::AxisChanged(Axis::RightStickX, value, _) => {
                    drone.rc_state.go_left_right(value)
                }
                EventType::AxisChanged(Axis::RightStickY, value, _) => {
                    drone.rc_state.go_forward_back(value)
                }
                EventType::ButtonPressed(Button::DPadDown, _) => {
                    drone.flip(Flip::Back).map_err(|_| "Flip failed")?;
                }
                EventType::ButtonPressed(Button::DPadUp, _) => {
                    drone.flip(Flip::Forward).map_err(|_| "Flip failed")?;
                }
                EventType::ButtonPressed(Button::DPadLeft, _) => {
                    drone.flip(Flip::Left).map_err(|_| "Flip failed")?;
                }
                EventType::ButtonPressed(Button::DPadRight, _) => {
                    drone.flip(Flip::Right).map_err(|_| "Flip failed")?;
                }
                _ => {}
            }
        };

        // poll drone state and react to it
        while let Some(msg) = drone.poll() {
            match msg {
                Message::Data(Package {data: PackageData::FlightData(_), ..}) => {
                    //println!("battery {}", d.battery_percentage);
                }
                Message::Data(d) /*if d.cmd != CommandIds::LogHeaderMsg*/ => {
                    println!("msg {:?}", d.clone());
                }
                Message::Frame(frame_id, d)=> {

                    socket.send_to(&d, "127.0.0.1:11110").expect("couldn't send data");
                    println!("send frame {} {:?}", frame_id, &d[..15]);
                }
                Message::Response(ResponseMsg::Connected(_)) => {
                    println!("connected");
                }
                _ => ()
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
