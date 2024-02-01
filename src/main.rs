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

    let (tx, mut rx, k, jh0) = tokio_serial::new(port_name, 9600)
        .open_native_async()
        .unwrap()
        .split();

    let _jh1 = tokio::spawn(async move {
        let stdin = stdin();

        let mut lines = BufReader::new(stdin).lines();

        loop {
            if let Ok(Some(line)) = lines.next_line().await {
                tx.send((line + "\n").as_bytes().into()).await.unwrap();
            } else {
                k.send(()).unwrap();
                break;
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Some(line) = rx.recv().await {
                if let Ok(line) = std::str::from_utf8(&line) {
                    print!("{}", line);
                } else {
                    eprintln!("got malformed packet");
                }
            }
        }
    });

    jh0.await??;

    Ok(())
}
