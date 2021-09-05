use std::time::Duration;
use tello::Drone;
use tokio::{select, time::sleep};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889").command_mode();
    // let mut drone = Drone::new("127.0.0.1:8880").command_mode();
    drone.enable().await?;

    let mut state = drone.state_receiver().unwrap();

    loop {
        let path = async {
            for _ in 0..6 {
                println!("forward {:?}", drone.forward(30).await);
                sleep(Duration::from_secs(5)).await;
                println!("cw {:?}", drone.cw(60).await);
                sleep(Duration::from_secs(4)).await;
            }
            println!("land {:?}", drone.land().await);
        };

        select! {
            _ = state.changed() => {
                if let Some(s) = state.borrow_and_update().clone() {
                    println!( "Battery {}% Height {}dm POS {:?}", s.bat, s.h, drone.odometry );
                }
            },
            _ = path => {
                println!("done");
                break;
            }
        }
    }
    sleep(Duration::from_secs(3)).await;
    Ok(())
}
