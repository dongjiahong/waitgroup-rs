use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(Semaphore::new(3));
    let mut join_handles = Vec::new();

    for i in 0..10 {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        join_handles.push(tokio::spawn(async move {
            println!("workr: {}", i);
            sleep(Duration::from_secs(1));
            drop(permit);
        }));
    }

    for handle in join_handles {
        handle.await.unwrap();
    }
}
