use pixel_widgets::prelude::*;
use winit::window::WindowBuilder;
use pixel_widgets::Command;
use futures::{SinkExt, FutureExt};

struct Download {
    pub state: ManagedState<String>,
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

impl Model for Download {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Vec<Command<Message>> {
        match message {
            Message::UrlChanged(url) => {
                self.url = url;
                Vec::new()
            }
            Message::DownloadPressed => {
                let (mut tx, rx) = futures::channel::mpsc::unbounded();
                let url = self.url.clone();
                vec![
                    Command::from_stream(rx),
                    Command::from_future_message(tokio::spawn(async move {
                        tx.send(Message::ProgressUpdated(0, 1)).await.unwrap();

                        let mut response = reqwest::get(reqwest::Url::parse(url.as_str()).unwrap()).await.unwrap();
                        let mut progress = 0;
                        let length = response.content_length().unwrap_or(0) as usize;

                        tx.send(Message::ProgressUpdated(0, length)).await.unwrap();
                        while let Ok(Some(bytes)) = response.chunk().await {
                            progress += bytes.len();
                            tx.send(Message::ProgressUpdated(progress, length)).await.unwrap();
                        }

                        Message::DownloadFinished
                    }).map(Result::unwrap))
                ]
            }
            Message::DownloadFinished => Vec::new(),
            Message::ProgressUpdated(downloaded, size) => {
                self.progress = downloaded;
                self.size = size;
                Vec::new()
            }
        }
    }

    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();
        let url = self.url.clone();
        Column::new()
            .push(Input::new(state.get_or_default_with("url", || {
                let mut state = pixel_widgets::widget::input::State::default();
                state.set_value(url);
                state
            }), "download link", Message::UrlChanged))
            .push(Button::new(state.get("download"), Text::new("Download")).on_clicked(Message::DownloadPressed))
            .push(Text::new(format!("Downloaded: {} / {} bytes", self.progress, self.size)))
            .push(Progress::new(self.progress as f32 / self.size as f32))
            .into_node()
    }
}

#[tokio::main]
async fn main() {
    let model = Download {
        state: Default::default(),
        progress: 0,
        size: 0,
        url: "http://speedtest.ftp.otenet.gr/files/test10Mb.db".into(),
    };

    let window = WindowBuilder::new()
        .with_title("Downloader")
        .with_inner_size(winit::dpi::LogicalSize::new(320, 240));

    let loader = pixel_widgets::loader::FsLoader::new("./examples".into()).unwrap();

    pixel_widgets::sandbox::run(model, loader, "download.pwss", window).await;
}
