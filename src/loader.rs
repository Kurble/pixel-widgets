use futures::channel::oneshot::*;
use futures::FutureExt;
use notify::event::{AccessKind, AccessMode, EventKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A way to load URLs from a data source.
///
/// Included implementations:
/// - `PathBuf`, loads data from disk using the `PathBuf` as working directory.
pub trait Loader: Send + Sync {
    /// A future returned when calling `load`.
    type Load: Future<Output = Result<Vec<u8>, Self::Error>> + Send;
    /// A future returned when calling `wait`.
    type Wait: Future<Output = Result<(), Self::Error>> + Send;
    /// Error returned by the loader when the request failed.
    type Error: 'static + Error + Send + Sync;
    /// Asynchronously load a resource located at the given url
    fn load(&self, url: impl AsRef<str>) -> Self::Load;
    /// Wait for a resource to be modified externally.
    fn wait(&self, url: impl AsRef<str>) -> Self::Wait;
}

/// Load file from the filesystem
pub struct FsLoader {
    base: PathBuf,
    _watcher: RecommendedWatcher,
    #[allow(clippy::type_complexity)]
    watches: Arc<Mutex<Vec<(PathBuf, Sender<Result<(), std::io::Error>>)>>>,
}

impl FsLoader {
    /// Construct a new FsLoader with the given base path
    pub fn new(base: PathBuf) -> Result<Self, std::io::Error> {
        let watches = Arc::new(Mutex::new(Vec::<(PathBuf, Sender<Result<(), std::io::Error>>)>::new()));

        let base = base.canonicalize()?;

        let mut watcher = notify::immediate_watcher({
            let watches = watches.clone();
            move |event: Result<notify::event::Event, notify::Error>| {
                if let Ok(notify::event::Event {
                    kind: EventKind::Access(AccessKind::Close(AccessMode::Write)),
                    paths,
                    ..
                }) = event
                {
                    let mut guard = watches.lock().unwrap();
                    *guard = guard
                        .drain(..)
                        .filter_map(|(path, sender)| {
                            if paths.contains(&path) {
                                sender.send(Ok(())).ok();
                                None
                            } else {
                                Some((path, sender))
                            }
                        })
                        .collect();
                }
            }
        })
        .expect("unable to create watcher");

        watcher
            .watch(&base, RecursiveMode::Recursive)
            .expect("failed to watch base path");

        Ok(Self {
            base,
            _watcher: watcher,
            watches,
        })
    }
}

impl Loader for FsLoader {
    type Load = futures::future::Ready<Result<Vec<u8>, Self::Error>>;
    #[allow(clippy::type_complexity)]
    type Wait = futures::future::Map<
        Receiver<Result<(), Self::Error>>,
        fn(Result<Result<(), Self::Error>, Canceled>) -> Result<(), Self::Error>,
    >;
    type Error = std::io::Error;

    fn load(&self, url: impl AsRef<str>) -> Self::Load {
        let path = self.base.join(std::path::Path::new(url.as_ref()));
        futures::future::ready(std::fs::read(path))
    }

    fn wait(&self, url: impl AsRef<str>) -> Self::Wait {
        let (tx, rx) = channel();

        match self.base.join(std::path::Path::new(url.as_ref())).canonicalize() {
            Ok(path) => {
                self.watches.lock().unwrap().push((path, tx));
            }
            Err(error) => {
                tx.send(Err(error)).ok();
            }
        }

        rx.map(|result| result.unwrap_or(Ok(())))
    }
}
