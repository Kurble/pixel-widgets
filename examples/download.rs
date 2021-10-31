use futures::SinkExt;
use winit::window::WindowBuilder;

use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;

#[derive(Default)]
struct Download;

struct DownloadState {
    pub url: String,
    pub progress: usize,
    pub size: usize,
}

#[derive(Clone)]
enum Message {
    UrlChanged(String),
    DownloadPressed,
    DownloadFinished,
    ProgressUpdated(usize, usize),
}

impl Component for Download {
    type Message = Message;
    type State = DownloadState;
    type Output = ();

    fn mount(&self) -> DownloadState {
        DownloadState {
            url: "http://speedtest.ftp.otenet.gr/files/test10Mb.db".into(),
            progress: 0,
            size: 0,
        }
    }

    fn view<'a>(&'a self, state: &'a Self::State) -> Node<'a, Self::Message> {
        view! {
            Column => {
                Input [
                    placeholder="download link",
                    val=state.url.as_str(),
                    on_change=Message::UrlChanged
                ],

                Button [
                    text="Download",
                    on_clicked=Message::DownloadPressed
                ],

                Text [val=format!("Downloaded: {} / {} bytes", state.progress, state.size)],
                Progress [val=state.progress as f32 / state.size as f32]
            }
        }
    }

    fn update(&self, message: Message, mut state: State<Self::State>, mut context: Context<Message, ()>) {
        match message {
            Message::UrlChanged(url) => {
                state.url = url;
            }
            Message::DownloadPressed => {
                let (mut tx, rx) = futures::channel::mpsc::unbounded();
                let url = state.url.clone();
                context.stream(rx);
                tokio::spawn(async move {
                    tx.send(Message::ProgressUpdated(0, 1)).await.unwrap();

                    let mut response = reqwest::get(reqwest::Url::parse(url.as_str()).unwrap()).await.unwrap();
                    let mut progress = 0;
                    let length = response.content_length().unwrap_or(0) as usize;

                    tx.send(Message::ProgressUpdated(0, length)).await.unwrap();
                    while let Ok(Some(bytes)) = response.chunk().await {
                        progress += bytes.len();
                        tx.send(Message::ProgressUpdated(progress, length)).await.unwrap();
                    }

                    tx.send(Message::DownloadFinished).await.unwrap();
                });
            }
            Message::DownloadFinished => (),
            Message::ProgressUpdated(downloaded, size) => {
                state.progress = downloaded;
                state.size = size;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Downloader")
        .with_inner_size(winit::dpi::LogicalSize::new(320, 240));

    let mut sandbox = Sandbox::new(Download, window).await;
    sandbox
        .ui
        .set_style(Style::from_file("examples/download.pwss").unwrap());

    sandbox.run().await;
}
