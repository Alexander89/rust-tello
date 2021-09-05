# Tello drone

There are two interfaces for the tello drone. The text based and a
non-public interface, used by the native app. The guys from the
[tellopilots forum](https://tellopilots.com/) did an awesome job by
reverse engineer this interface and support other public repositories
for go, python...

This library combines the network protocol to communicate with the drone and get
available meta data additionally and a remote-control framework is available to
simplify the wiring to the keyboard or an joystick.

In the sources you will find an example, how to create a SDL-Ui and use
the keyboard to control the drone. You can run it with `cargo run --example fly`

**Please keep in mind, advanced maneuvers require a bright environment. (Flip, Bounce, ...)**

## Communication

When the drone gets an enable package (`drone.connect(11111);`), the Tello drone
send data on two UDP channels. A the command channel (port: 8889) and B (WIP) the
video channel (default: port: 11111). In the AP mode, the drone will appear with
the default ip 192.168.10.1. All send calls are done synchronously.
To receive the data, you have to poll the drone. Here is an example:

### Example

```rust
use tello::{Drone, Message, Package, PackageData, ResponseMsg};
use std::time::Duration;

fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889");
    drone.connect(11111);
    loop {
        if let Some(msg) = drone.poll() {
            match msg {
                Message::Data(Package {data: PackageData::FlightData(d), ..}) => {
                    println!("battery {}", d.battery_percentage);
                }
                Message::Response(ResponseMsg::Connected(_)) => {
                    println!("connected");
                    drone.throw_and_go().unwrap();
                }
                _ => ()
            }
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}
```

## Command mode

You can switch the drone to the command mode. to get back to the "Free-Flight-Mode" you have to reboot the drone.
The CommandMode provides following information to you:

-   `state_receiver(): Option<Receiver<CommandModeState>>`: parsed incoming state packages from the drone. You will take the ownership, you could do this only once.
-   `video_receiver(): Option<Receiver<Vec<u8>>>`: Video frames (h264) from the drone. You will take the ownership, you could do this only once.
-   `odometry: Odometry` odometer data for your movements.

### Example

```rust
use futures::executor::block_on;
use std::{string::String, thread::sleep, time::Duration};
use tello::Drone;

fn main() -> Result<(), String> {
    block_on(async {
        let mut drone = Drone::new("192.168.10.1:8889").command_mode();
        let state = drone.state_receiver().unwrap();
        drone.enable().await?;

        match state.recv_timeout(Duration::from_secs(5)) {
            Ok(message) => println!(
                "Battery {}% Height {}dm POS {:?}",
                message.bat, message.h, drone.odometry
            ),
            _ => println!("No state package received"),
        }

        println!("take_off {:?}", drone.take_off().await);
        sleep(Duration::from_secs(7));

        for _ in 0..6 {
            println!("forward {:?}", drone.forward(30).await);
            sleep(Duration::from_secs(5));
            println!("cw {:?}", drone.cw(60).await);
            sleep(Duration::from_secs(4));
        }

        println!("land {:?}", drone.land().await);
        sleep(Duration::from_secs(3));
        Ok(())
    })
}
```

## Remote control

the poll is not only receiving messages from the drone, it will also send some default-settings, replies with acknowledgements, triggers the key frames or send the remote-control state for the live move commands.

The Drone contains a rc_state to manipulate the movement. e.g.: `drone.rc_state.go_down()`, `drone.rc_state.go_forward_back(-0.7)`

The following example is opening a window with SDL, handles the keyboard inputs and shows how to connect a game pad or joystick.

### Example

```rust
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use tello::{Drone, Message, Package, PackageData, ResponseMsg};
use std::time::Duration;

fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889");
    drone.connect(11111);

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem.window("TELLO drone", 1280, 720).build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        // draw some stuff
        canvas.clear();
        // [...]

        // handle input from a keyboard or something like a game-pad
        // ue the keyboard events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } =>
                    break 'running,
                Event::KeyDown { keycode: Some(Keycode::K), .. } =>
                    drone.take_off().unwrap(),
                Event::KeyDown { keycode: Some(Keycode::L), .. } =>
                    drone.land().unwrap(),
                Event::KeyDown { keycode: Some(Keycode::A), .. } =>
                    drone.rc_state.go_left(),
                Event::KeyDown { keycode: Some(Keycode::D), .. } =>
                    drone.rc_state.go_right(),
                Event::KeyUp { keycode: Some(Keycode::A), .. }
                | Event::KeyUp { keycode: Some(Keycode::D), .. } =>
                    drone.rc_state.stop_left_right(),
                //...
            }
        }

        // or use a game pad (range from -1 to 1)
        // drone.rc_state.go_left_right(dummy_joystick.axis.1);
        // drone.rc_state.go_forward_back(dummy_joystick.axis.2);
        // drone.rc_state.go_up_down(dummy_joystick.axis.3);
        // drone.rc_state.turn(dummy_joystick.axis.4);

        // the poll will send the move command to the drone
        drone.poll();

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}
```
