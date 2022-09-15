// use tokio::{net::TcpStream};

// struct Connection {
//     stream: TcpStream,
//     signal: String,
// }

// impl Connection {
//     async fn new() -> Connection {
//         let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
//         Connection {
//             stream,
//             signal: String::from(""),
//         }
//     }
// }

// pub async fn connection() {
//     let c = Connection::new().await;

//     loop {
//         println!("live")
//     }


// }

// pub async fn disconnect_signal() {
//     println!("disconnect signal");
// }

// pub async fn quit_signal() {
//     println!("quit signal");
// }

