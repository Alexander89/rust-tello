use futures::executor::block_on;
use std::{string::String, thread::sleep, time::Duration};
use tello::Drone;

fn main() -> Result<(), String> {
    block_on(async {
        let mut drone = Drone::new("192.168.10.1:8889").command_mode();
        let _failed_sometimes_but_works = drone.enable().await;

        match drone.state_receiver.recv_timeout(Duration::from_secs(5)) {
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
    });
    Ok(())
}
