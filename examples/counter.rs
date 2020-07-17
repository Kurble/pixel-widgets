use maple::prelude::*;

mod framework;

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
            .push(Button::new(state.get("down"), Text::new("Down")).on_clicked(Message::DownPressed))
            .into_node()
    }
}

fn main() {
    framework::run_model(
        Counter {
            value: 0,
            state: ManagedState::default(),
        },
        "counter.mss",
    );
}
