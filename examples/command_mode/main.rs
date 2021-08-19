use futures::executor::block_on;
use std::{string::String, thread::sleep, time::Duration};
use tello::Drone;

fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889").command_mode();

    sleep(Duration::from_millis(100));
    block_on(drone.enable())?;

    match drone.state_receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(message) => println!(
            "Battery {}% Height {}dm POS {:?}",
            message.bat, message.h, drone.position
        ),
        _ => println!("No state package received"),
    }

    block_on(drone.take_off())?;
    sleep(Duration::from_secs(1));

    for _ in 0..3 {
        block_on(drone.forward(50))?;
        sleep(Duration::from_secs(1));
        block_on(drone.cw(90))?;
        sleep(Duration::from_secs(1));
    }

    block_on(drone.land())?;
    sleep(Duration::from_secs(1));

    Ok(())
}
