use std::{
    hash::Hash,
    io::{self, Result},
};

use rustc_hash::FxHashMap;
use tokio::{
    io::{Interest, unix::AsyncFd},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::AbortHandle,
};

use crate::{Event, PsiFd};

pub struct PsiTokioReactor<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    rx: UnboundedReceiver<Event<T>>,
    inner: Box<[Option<PsiTokioReactorInner<T>>]>,
    abort_handles: Option<Box<[AbortHandle]>>,
}

impl<T> PsiTokioReactor<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub(crate) fn new(map: FxHashMap<T, PsiFd>) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let inner: Box<[PsiTokioReactorInner<T>]> = map
            .into_iter()
            .map(|(id, fd)| PsiTokioReactorInner::new(id, fd, tx.clone()))
            .collect::<Result<_>>()?;
        Ok(Self {
            rx,
            inner: inner.into_iter().map(Some).collect(),
            abort_handles: None,
        })
    }

    /// Spawn all tasks
    pub fn start(&mut self) -> Result<()> {
        self.abort_handles = self
            .inner
            .iter_mut()
            .map(|i| i.take().map(|i| tokio::spawn(i.run()).abort_handle()))
            .collect();
        Ok(())
    }

    /// Receive an event from tasks
    pub async fn recv(&mut self) -> Result<Event<T>> {
        if self.abort_handles.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "the tokio tasks is not started",
            ));
        }
        self.rx.recv().await.ok_or(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "called a recv on a closed channel",
        ))
    }
}

impl<T> Drop for PsiTokioReactor<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn drop(&mut self) {
        if let Some(h) = self.abort_handles.take() {
            h.iter().for_each(|h| h.abort())
        }
    }
}

struct PsiTokioReactorInner<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    id: T,
    fd: AsyncFd<PsiFd>,
    tx: UnboundedSender<Event<T>>,
}

impl<T> PsiTokioReactorInner<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn new(id: T, fd: PsiFd, tx: UnboundedSender<Event<T>>) -> Result<Self> {
        let fd = AsyncFd::with_interest(fd, Interest::PRIORITY)?;
        Ok(Self { id, fd, tx })
    }

    async fn run(self) {
        loop {
            match self.fd.readable().await {
                Ok(mut guard) => {
                    if self.tx.send(Event::Ready(self.id.clone())).is_err() {
                        break;
                    };
                    guard.clear_ready();
                }
                Err(e) => {
                    if self.tx.send(Event::Failure(e)).is_err() {
                        break;
                    };
                }
            }
        }
    }
}
