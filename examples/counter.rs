use pixel_widgets::prelude::*;
use std::path::PathBuf;
use winit::window::WindowBuilder;

struct Counter {
    pub value: i32,
    pub state: ManagedState<String>,
}

enum Message {
    UpPressed,
    DownPressed,
}

impl Model for Counter {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::UpPressed => {
                self.value += 1;
            }
            Message::DownPressed => {
                self.value -= 1;
            }
        }
    }

    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();
        Column::new()
            .push(Button::new(state.get("up"), Text::new("Up")).on_clicked(Message::UpPressed))
            .push(Text::new(format!("Count: {}", self.value)))
            .push(Text::new("Test 1"))
            .push(Text::new("Test 2"))
            .push(Text::new("Test 3"))
            .push(Text::new("Test 4"))
            .push(Button::new(state.get("down"), Text::new("Down")).on_clicked(Message::DownPressed))
            .into_node()
    }
}

fn main() {
    let model = Counter {
        value: 0,
        state: ManagedState::default(),
    };

    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    pixel_widgets::sandbox::run(model, PathBuf::from("./examples"), "counter.pwss", window);
}
