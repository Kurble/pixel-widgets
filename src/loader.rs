use std::future::Future;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use notify::event::{EventKind, AccessKind, AccessMode};
use futures::channel::oneshot::*;
use futures::FutureExt;

/// A way to load URLs from a data source.
///
/// Included implementations:
/// - `PathBuf`, loads data from disk using the `PathBuf` as working directory.
pub trait Loader: 'static + Send + Sync {
    /// A future returned when calling `load`.
    type Load: Future<Output = Result<Vec<u8>, Self::Error>> + Send + Sync;
    /// A future returned when calling `wait`.
    type Wait: Future<Output = ()> + Send + Sync;
    /// Error returned by the loader when the request failed.
    type Error: Error + Send;
    /// Asynchronously load a resource located at the given url
    fn load(&self, url: impl AsRef<str>) -> Self::Load;
    /// Wait for a resource to be modified externally.
    fn wait(&self, url: impl AsRef<str>) -> Self::Wait;
}

/// Load file from the filesystem
pub struct FsLoader {
    base: PathBuf,
    _watcher: RecommendedWatcher,
    watches: Arc<Mutex<Vec<(PathBuf, Sender<()>)>>>,
}

impl FsLoader {
    /// Construct a new FsLoader with the given base path
    pub fn new(base: PathBuf) -> Self {
        let watches = Arc::new(Mutex::new(Vec::<(PathBuf, Sender<()>)>::new()));

        let base = base.canonicalize().expect("unable to create absolute path");

        let mut watcher = notify::immediate_watcher({
            let watches = watches.clone();
            move |event: Result<notify::event::Event, notify::Error>| {
                match event {
                    Ok(event) => {
                        match event.kind {
                            EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                                let mut guard = watches.lock().unwrap();
                                *guard = guard.drain(..).filter_map(|(path, sender)| {
                                    if event.paths.contains(&path) {
                                        sender.send(()).ok();
                                        None
                                    } else {
                                        Some((path, sender))
                                    }
                                }).collect();
                            }
                            _ => (),
                        }
                    },
                    Err(_) => (),
                }
            }
        }).expect("unable to create watcher");

        watcher.watch(&base, RecursiveMode::Recursive).expect("failed to watch base path");

        Self {
            base,
            _watcher: watcher,
            watches,
        }
    }
}

impl Loader for FsLoader {
    type Load = futures::future::Ready<Result<Vec<u8>, Self::Error>>;
    type Wait = futures::future::Map<Receiver<()>, fn(Result<(), Canceled>) -> ()>;
    type Error = std::io::Error;

    fn load(&self, url: impl AsRef<str>) -> Self::Load {
        let path = self.base.join(std::path::Path::new(url.as_ref()));
        futures::future::ready(std::fs::read(path))
    }

    fn wait(&self, url: impl AsRef<str>) -> Self::Wait {
        let path = self.base.join(std::path::Path::new(url.as_ref())).canonicalize().expect("path not found");
        let (tx, rx) = channel();
        self.watches.lock().unwrap().push((path, tx));
        rx.map(|_|())
    }
}
