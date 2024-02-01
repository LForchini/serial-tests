use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_serial::SerialStream;

pub trait SerialStreamExt {
    fn split(self) -> (Sender<Box<[u8]>>, Receiver<Box<[u8]>>);
}

impl SerialStreamExt for SerialStream {
    fn split(self) -> (Sender<Box<[u8]>>, Receiver<Box<[u8]>>) {
        let (in_tx, in_rx) = channel(10);
        let (out_tx, out_rx) = channel(10);

        let mut sss = SplitSerialStream {
            in_tx,
            in_rx,
            out_tx,
            out_rx: Some(out_rx),
            inner: self,
        };

        let tx = sss.get_tx();
        let rx = sss.get_rx().unwrap();

        tokio::spawn(async move { sss.start().await });

        (tx, rx)
    }
}

pub struct SplitSerialStream {
    in_tx: Sender<Box<[u8]>>,
    in_rx: Receiver<Box<[u8]>>,
    out_tx: Sender<Box<[u8]>>,
    out_rx: Option<Receiver<Box<[u8]>>>,

    inner: SerialStream,
}

impl SplitSerialStream {
    pub fn get_tx(&self) -> Sender<Box<[u8]>> {
        self.in_tx.clone()
    }

    pub fn get_rx(&mut self) -> Option<Receiver<Box<[u8]>>> {
        self.out_rx.take()
    }

    pub async fn start(&mut self) -> Result<(), SplitSerialStreamError<Box<[u8]>>> {
        let mut buf = vec![0; 1024];

        loop {
            tokio::select! {
                Some(data) = self.in_rx.recv() => {
                    self.inner.write_all(&data).await?;
                }
                Ok(bytes) = self.inner.read(&mut buf) => {
                    let buf = &buf[..bytes];
                    self.out_tx.send(buf.into()).await?;
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SplitSerialStreamError<E> {
    #[error("io error")]
    IOError(#[from] std::io::Error),

    #[error("stream error")]
    StreamSendError(#[from] tokio::sync::mpsc::error::SendError<E>),
}
