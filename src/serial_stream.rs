use std::time::Duration;

use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};
use tokio_serial::SerialStream;

pub trait SerialStreamExt {
    fn split(
        self,
    ) -> (
        Sender<Box<[u8]>>,
        Receiver<Box<[u8]>>,
        oneshot::Sender<()>,
        JoinHandle<Result<(), SplitSerialStreamError<Box<[u8]>>>>,
    );
}

impl SerialStreamExt for SerialStream {
    fn split(
        self,
    ) -> (
        Sender<Box<[u8]>>,
        Receiver<Box<[u8]>>,
        oneshot::Sender<()>,
        JoinHandle<Result<(), SplitSerialStreamError<Box<[u8]>>>>,
    ) {
        let (in_tx, in_rx) = channel(1024);
        let (out_tx, out_rx) = channel(1024);
        let (kill_tx, kill_rx) = oneshot::channel();

        let mut sss = SplitSerialStream {
            in_tx,
            in_rx,

            out_tx,
            out_rx: Some(out_rx),

            kill_tx: Some(kill_tx),
            kill_rx,

            inner: self,
        };

        let tx = sss.get_tx();
        let rx = sss.get_rx().unwrap();
        let kill = sss.get_kill().unwrap();

        let jh = tokio::spawn(async move { sss.start().await });

        (tx, rx, kill, jh)
    }
}

pub struct SplitSerialStream {
    in_tx: Sender<Box<[u8]>>,
    in_rx: Receiver<Box<[u8]>>,

    out_tx: Sender<Box<[u8]>>,
    out_rx: Option<Receiver<Box<[u8]>>>,

    kill_tx: Option<oneshot::Sender<()>>,
    kill_rx: oneshot::Receiver<()>,

    inner: SerialStream,
}

impl SplitSerialStream {
    pub fn get_tx(&self) -> Sender<Box<[u8]>> {
        self.in_tx.clone()
    }

    pub fn get_rx(&mut self) -> Option<Receiver<Box<[u8]>>> {
        self.out_rx.take()
    }

    pub fn get_kill(&mut self) -> Option<oneshot::Sender<()>> {
        self.kill_tx.take()
    }

    pub async fn start(&mut self) -> Result<(), SplitSerialStreamError<Box<[u8]>>> {
        let mut buf = vec![0; 1024];

        loop {
            tokio::select! {
                _ = &mut self.kill_rx => {
                    break;
                }
                Some(data) = self.in_rx.recv() => {
                    self.inner.write_all(&data).await?;
                }
                Ok(bytes) = self.inner.read(&mut buf) => {
                    let buf = &buf[..bytes];

                    self.out_tx.send(buf.into()).await?;
                }
            }
        }

        while let Ok(data) = self.in_rx.try_recv() {
            println!("{:?}", data);
            self.inner.write_all(&data).await?;
        }

        // there needs to be some delay here...

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SplitSerialStreamError<E> {
    #[error("io error")]
    IOError(#[from] std::io::Error),

    #[error("stream error")]
    StreamSendError(#[from] tokio::sync::mpsc::error::SendError<E>),

    #[error("stream error")]
    KillError(#[from] tokio::sync::oneshot::error::RecvError),
}
