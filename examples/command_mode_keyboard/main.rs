use futures::executor::block_on;
use std::string::String;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::sleep;
use std::time::Duration;
use std::{io, thread};

use tello::Drone;

fn main() -> Result<(), String> {
    block_on(async {
        let mut drone = Drone::new("192.168.10.1:8889").command_mode();
        let state = drone.state_receiver().unwrap();

        let stdin_channel = create_stdin_channel();
        let _failed_sometimes_but_works = drone.enable().await;
        'mainLoop: loop {
            match stdin_channel.try_recv() {
                Ok(input) => {
                    let commands: Vec<&str> = input.split(' ').collect();
                    let send = match commands[0] {
                        "exit" => break 'mainLoop,
                        "streamon" => drone.video_on().await,
                        "streamoff" => drone.video_off().await,
                        "enable" => drone.enable().await,
                        "start" => drone.take_off().await,
                        "land" => drone.land().await,
                        "down" => drone.down(commands[1].parse().unwrap_or(0)).await,
                        "up" => drone.up(commands[1].parse().unwrap_or(0)).await,
                        "forward" => drone.forward(commands[1].parse().unwrap_or(0)).await,
                        "back" => drone.back(commands[1].parse().unwrap_or(0)).await,
                        "left" => drone.left(commands[1].parse().unwrap_or(0)).await,
                        "right" => drone.right(commands[1].parse().unwrap_or(0)).await,
                        "cw" => drone.cw(commands[1].parse().unwrap_or(0)).await,
                        "ccw" => drone.ccw(commands[1].parse().unwrap_or(0)).await,
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
            match state.try_recv() {
                Ok(message) => println!(
                    "battery {}%  height {}dm POS {:?}",
                    message.bat, message.h, drone.odometry
                ),
                _ => (),
            }
            sleep(Duration::from_millis(100));
        }
        Ok(())
    })
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
