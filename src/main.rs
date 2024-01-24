mod serial_stream;

use std::env;

use serial_stream::SerialStreamExt;
use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio_serial::SerialPortBuilderExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ports = tokio_serial::available_ports().expect("No ports found!");

    if ports.len() == 0 {
        panic!("no serial ports found")
    }

    let port_name = &env::args().collect::<Vec<_>>()[1];

    let (tx, mut rx) = tokio_serial::new(port_name, 19_200)
        .open_native_async()
        .unwrap()
        .split();

    let jh1 = tokio::spawn(async move {
        let stdin = stdin();
        let mut lines = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            tx.send(line.as_bytes().into()).await.unwrap();
        }
    });

    let jh2 = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            println!("{:?}", std::str::from_utf8(&line));
        }
    });

    tokio::select!(
        _ = jh1 => {}
        _ = jh2 => {}
    );

    Ok(())
}
