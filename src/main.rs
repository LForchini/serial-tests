use std::{env, time::Duration};

use std::fmt::Debug;
use tokio::io::{stdin, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio_serial::SerialPortBuilderExt;
use tracing::{info, instrument, trace, warn};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_tree::HierarchicalLayer;

#[instrument]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stderr)
        .with_indent_lines(true)
        .with_indent_amount(2)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_verbose_exit(false)
        .with_verbose_entry(true)
        .with_targets(true);

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let ports = tokio_serial::available_ports().expect("Ports should be readable");

    if ports.len() == 0 {
        panic!("no serial ports found")
    }

    let port_name = &env::args().collect::<Vec<_>>()[1];

    let (read, write) = tokio::io::split(
        tokio_serial::new(port_name, 9600)
            .open_native_async()
            .unwrap(),
    );

    let mut br = BufReader::new(read).lines();
    let mut bw = BufWriter::new(write);

    let mut stdin = BufReader::new(stdin()).lines();

    info!("entering ready state");
    loop {
        tokio::select! {
            Ok(Some(line)) = br.next_line() => {
                handle_read(line).await;
            }

            Ok(opt_line) = stdin.next_line() => {
                match opt_line {
                    Some(line) => {handle_write(&mut bw, line).await;},
                    None => {break;}
                }
            }
        }
    }

    bw.flush().await.unwrap();
    trace!(?bw, "flushed");

    Ok(())
}

#[instrument]
async fn handle_write<T>(bw: &mut BufWriter<T>, line: String)
where
    T: Debug + AsyncWrite + AsyncWriteExt + Unpin,
{
    bw.write_all((line + "\n").as_bytes()).await.unwrap();
}

#[instrument]
async fn handle_read(line: String) {
    println!("{line}");
}
