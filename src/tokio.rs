use std::{
    hash::Hash,
    io::{self, Result},
    sync::Mutex,
};

use rustc_hash::FxHashMap;
use tokio::{
    io::{Interest, unix::AsyncFd},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::AbortHandle,
};

use crate::{Event, PsiFd};

/// The core state of a PSI Tokio reactor.
struct ReactorCore<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    rx: UnboundedReceiver<Event<T>>,
    tx: UnboundedSender<Event<T>>,
    handles: Mutex<FxHashMap<T, AbortHandle>>,
}

impl<T> ReactorCore<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn add_fd(&mut self, id: T, fd: PsiFd) -> Result<()> {
        if !fd.from_builder {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PsiFd added to an active reactor must be created using PsiFdBuilder",
            ));
        }

        let mut handles = self.handles.lock().unwrap();
        if handles.contains_key(&id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "PsiFd with the same ID already exists",
            ));
        }

        let inner = PsiTokioReactorInner::new(id.clone(), fd, self.tx.clone())?;
        let handle = tokio::spawn(inner.run()).abort_handle();
        handles.insert(id, handle);
        Ok(())
    }

    fn remove_fd(&mut self, id: &T) -> bool {
        if let Some(handle) = self.handles.lock().unwrap().remove(id) {
            handle.abort();
            true
        } else {
            false
        }
    }
}

impl<T> Drop for ReactorCore<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn drop(&mut self) {
        self.handles
            .lock()
            .unwrap()
            .values()
            .for_each(|h| h.abort());
    }
}

/// A PSI reactor that has been initialized with file descriptors but not yet started.
pub struct PsiTokioReactorPending<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    core: ReactorCore<T>,
    pending: Vec<PsiTokioReactorInner<T>>,
}

impl<T> PsiTokioReactorPending<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub(crate) fn new(map: FxHashMap<T, PsiFd>) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        let pending: Vec<PsiTokioReactorInner<T>> = map
            .into_iter()
            .map(|(id, fd)| PsiTokioReactorInner::new(id, fd, tx.clone()))
            .collect::<Result<_>>()?;

        Ok(Self {
            core: ReactorCore {
                rx,
                tx,
                handles: Mutex::new(FxHashMap::default()),
            },
            pending,
        })
    }

    /// Spawn all initial tasks and consume self to transition to the Active state.
    pub fn start(mut self) -> Result<PsiTokioReactorActive<T>> {
        if let Ok(mut handles) = self.core.handles.lock() {
            for inner in self.pending.drain(..) {
                let id = inner.id.clone();
                let task_handle = tokio::spawn(inner.run()).abort_handle();
                handles.insert(id, task_handle);
            }
        }

        Ok(PsiTokioReactorActive { core: self.core })
    }
}

/// A fully active PSI reactor listening for events.
pub struct PsiTokioReactorActive<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    core: ReactorCore<T>,
}

impl<T> PsiTokioReactorActive<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    /// Add a `PsiFd` to the reactor. This will immediately spawn a monitoring task.
    pub fn add_fd(&mut self, id: T, fd: PsiFd) -> Result<()> {
        self.core.add_fd(id, fd)
    }

    /// Remove a `PsiFd` from the reactor by its ID.
    pub fn remove_fd(&mut self, id: &T) -> bool {
        self.core.remove_fd(id)
    }

    /// Receive an event from the monitoring tasks.
    pub async fn recv(&mut self) -> Result<Event<T>> {
        self.core.rx.recv().await.ok_or(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "called recv on a closed channel",
        ))
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
                    }
                    guard.clear_ready();
                }
                Err(e) => {
                    if self.tx.send(Event::Failure(e)).is_err() {
                        break;
                    }
                }
            }
        }
    }
}
