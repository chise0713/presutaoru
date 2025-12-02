use std::{
    hash::Hash,
    io::{self, Result},
    mem::MaybeUninit,
    os::fd::AsFd,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

use nix::{
    errno::Errno::{EAGAIN, EBADF, EINTR},
    sys::{
        epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout},
        eventfd::{EfdFlags, EventFd},
    },
};
use rustc_hash::FxHashMap;

use crate::{Event, PsiFd};

pub struct PsiThread<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    handle: Option<JoinHandle<()>>,
    rx: Receiver<Event<T>>,
    inner: Arc<PsiThreadInner<T>>,
    // do not drop the owned fd, or epoll will just stall
    _fds: Box<[PsiFd]>,
    efd: EventFd,
}

impl<T> PsiThread<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub(crate) fn new(map: FxHashMap<T, PsiFd>) -> Result<Self> {
        let epfd = Epoll::new(EpollCreateFlags::empty())?;
        let (tx, rx) = mpsc::channel();
        let len = map.len();
        let mut _fds = Box::new_uninit_slice(len);
        let mut ids = Box::new_uninit_slice(len);
        for (i, (k, fd)) in map.into_iter().enumerate() {
            epfd.add(fd.as_fd(), EpollEvent::new(EpollFlags::EPOLLPRI, i as u64))?;
            ids[i].write(k);
            _fds[i].write(fd);
        }
        let efd = EventFd::from_flags(EfdFlags::EFD_NONBLOCK)?;
        epfd.add(efd.as_fd(), EpollEvent::new(EpollFlags::EPOLLIN, u64::MAX))?;
        Ok(Self {
            handle: None,
            rx,
            inner: Arc::new(PsiThreadInner {
                epfd,
                tx,
                ids: unsafe { ids.assume_init() },
            }),
            _fds: unsafe { _fds.assume_init() },
            efd,
        })
    }

    /// Start the thread in background
    pub fn start(&mut self) -> io::Result<()> {
        self.handle = Some(
            thread::Builder::new()
                .name("PsiMonitorThread".to_owned())
                .spawn({
                    let inner = self.inner.clone();
                    move || _ = inner.as_ref().epoll_loop()
                })?,
        );
        Ok(())
    }

    /// Receive an event from the thread
    pub fn recv(&self) -> Result<Event<T>> {
        if self.handle.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "the epoll thread is not started",
            ));
        }
        self.rx.recv().map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "called a recv on a closed channel",
            )
        })
    }
}

impl<T> Drop for PsiThread<T>
where
    T: Hash + Eq + Clone + Send + Sync,
{
    fn drop(&mut self) {
        // to notify the thread to exit
        _ = self.efd.write(1).unwrap();
        if let Some(h) = self.handle.take() {
            _ = h.join()
        };
    }
}

struct PsiThreadInner<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    epfd: Epoll,
    tx: Sender<Event<T>>,
    ids: Box<[T]>,
}

impl<T> PsiThreadInner<T>
where
    T: Hash + Eq + Clone + Send + Sync + 'static,
{
    #[inline(always)]
    pub(crate) fn epoll_loop(&self) -> io::Result<()> {
        let send = |item: Event<T>| {
            self.tx
                .send(item)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))
        };
        let mut events = Box::new_uninit_slice(self.ids.len());
        events.fill(MaybeUninit::new(EpollEvent::empty()));
        let mut events = unsafe { events.assume_init() };
        loop {
            match self.epfd.wait(&mut events, EpollTimeout::NONE) {
                Ok(num) => {
                    for ev in &events[..num] {
                        if ev.data() == u64::MAX {
                            return Ok(());
                        }
                        let id = self.ids[ev.data() as usize].clone();
                        send(Event::Ready(id))?;
                    }
                }
                // interrupted, try again
                Err(EINTR | EAGAIN) => {
                    continue;
                }
                // epoll fd closed, exit thread
                Err(EBADF) => {
                    return Ok(());
                }
                Err(e) => {
                    send(Event::Failure(e.into()))?;
                    return Err(e.into());
                }
            }
        }
    }
}
