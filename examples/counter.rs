use pixel_widgets::prelude::*;
use winit::window::WindowBuilder;
use pixel_widgets::Command;

struct Counter {
    pub value: i32,
    pub state: ManagedState<String>,
}

#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

impl Model for Counter {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Vec<Command<Message>> {
        match message {
            Message::UpPressed => {
                self.value += 1;
                Vec::new()
            }
            Message::DownPressed => {
                self.value -= 1;
                Vec::new()
            }
        }
    }

    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();
        Column::new()
            .push(Button::new(state.get("up"), Text::new("Up")).on_clicked(Message::UpPressed))
            .push(Text::new(format!("Count: {}", self.value)))
            .push(Button::new(state.get("down"), Text::new("Down")).on_clicked(Message::DownPressed))
            .into_node()
    }
}

#[tokio::main]
async fn main() {
    let model = Counter {
        value: 0,
        state: ManagedState::default(),
    };

    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    let loader = pixel_widgets::loader::FsLoader::new("./examples".into()).unwrap();

    let mut sandbox = Sandbox::new(model, loader, window).await;
    sandbox.ui.set_stylesheet("counter.pwss").await.unwrap();
    sandbox.run().await;
}
