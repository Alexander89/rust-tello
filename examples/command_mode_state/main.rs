use tello::Drone;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889").command_mode();
    // let mut drone = Drone::new("127.0.0.1:8880").command_mode();
    drone.enable().await?;
    let mut state = drone.state_receiver().unwrap();

    loop {
        if let Ok(_) = state.changed().await {
            if let Some(s) = state.borrow_and_update().clone() {
                println!(
                    "Battery {}% Height {}dm | pitch {}° roll {}° yaw {}° | baro {}",
                    s.bat, s.h, s.pitch, s.roll, s.yaw, s.baro
                );
            }
        }
    }
}
