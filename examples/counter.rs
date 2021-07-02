use pixel_widgets::prelude::*;
use std::sync::Arc;
use winit::window::WindowBuilder;

struct Counter;

#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

impl Component for Counter {
    type Message = Message;
    type State = i32;
    type Output = ();

    fn mount(&self) -> Self::State {
        15
    }

    fn view(&self, state: &i32) -> Node<Message> {
        Column::new()
            .push(Button::new(Text::new("Up")).on_clicked(Message::UpPressed))
            .push(Text::new(format!("Count: {}", *state)))
            .push(Button::new(Text::new("Down")).on_clicked(Message::DownPressed))
            .into_node()
    }

    fn update(&self, message: Self::Message, state: &mut i32) -> Vec<Self::Output> {
        match message {
            Message::UpPressed => {
                *state += 1;
                Vec::new()
            }
            Message::DownPressed => {
                *state -= 1;
                Vec::new()
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    let loader = pixel_widgets::loader::FsLoader::new("./examples".into()).unwrap();

    let mut sandbox = Sandbox::new(Counter, window).await;

    let style = Arc::new(Style::load(&loader, "counter.pwss", 512, 0).await.unwrap());
    sandbox.ui.set_style(style);

    sandbox.run().await;
}
