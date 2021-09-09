use futures::StreamExt;
use tello::Drone;
use tokio_stream::wrappers::WatchStream;

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut drone = Drone::new("192.168.10.1:8889").command_mode();
    drone.enable().await?;
    let state = WatchStream::new(drone.state_receiver().unwrap());

    let path = async {
        println!("take off {:?}", drone.take_off().await);
        for _ in 0..6 {
            println!("forward {:?}", drone.forward(30).await);
            println!("cw {:?}", drone.cw(60).await);
        }
        println!("land {:?}", drone.land().await);
    };

    let mut s = Box::pin(state.take_until(path));
    while let Some(s) = s.next().await {
        if let Some(state) = s {
            println!("Battery {}% Height {}", state.bat, state.h);
        }
    }
    Ok(())
}
