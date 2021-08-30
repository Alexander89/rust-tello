use futures::executor::block_on;
use std::string::String;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::sleep;
use std::time::Duration;
use std::{io, thread};

use tello::Drone;

fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889").command_mode();

    let stdin_channel = create_stdin_channel();
    let _failed_sometimes_but_works = block_on(drone.enable());
    block_on(drone.video_on())?;
    // block_on(drone.video_off());
    'mainLoop: loop {
        match stdin_channel.try_recv() {
            Ok(input) => {
                let commands: Vec<&str> = input.split(' ').collect();
                let send = match commands[0] {
                    "exit" => break 'mainLoop,
                    "streamon" => block_on(drone.video_on()),
                    "streamoff" => block_on(drone.video_off()),
                    "enable" => block_on(drone.enable()),
                    "start" => block_on(drone.take_off()),
                    "land" => block_on(drone.land()),
                    "down" => block_on(drone.down(commands[1].parse().unwrap_or(0))),
                    "up" => block_on(drone.up(commands[1].parse().unwrap_or(0))),
                    "forward" => block_on(drone.forward(commands[1].parse().unwrap_or(0))),
                    "back" => block_on(drone.back(commands[1].parse().unwrap_or(0))),
                    "left" => block_on(drone.left(commands[1].parse().unwrap_or(0))),
                    "right" => block_on(drone.right(commands[1].parse().unwrap_or(0))),
                    "cw" => block_on(drone.cw(commands[1].parse().unwrap_or(0))),
                    "ccw" => block_on(drone.ccw(commands[1].parse().unwrap_or(0))),
                    //"home" => drone.go_home(0, 0, 0, 20),
                    _ => Ok(()),
                };
                if send.is_err() {
                    println!("{}", send.err().unwrap())
                }
            }
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
            _ => (),
        }
        match drone.state_receiver.try_recv() {
            Ok(message) => println!(
                "battery {}%  height {}dm POS {:?}",
                message.bat, message.h, drone.odometry
            ),
            _ => (),
        }
        sleep(Duration::from_millis(100));
    }
    Ok(())
}

fn create_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer.trim().to_string()).unwrap();
    });
    rx
}
