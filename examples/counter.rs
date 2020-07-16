use maple::element::Node;
use maple::tracker::ManagedState;
use maple::*;

mod framework;

struct Counter {
    pub value: i32,
    pub name: String,
    pub password: String,
    pub state: ManagedState<String>,
}

enum Message {
    UpPressed,
    DownPressed,
    NameChanged(String),
    PasswordChanged(String),
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
            Message::NameChanged(name) => {
                self.name = name;
            }
            Message::PasswordChanged(password) => {
                self.password = password;
            }
        }
    }

    fn view(&mut self) -> Node<Message> {
        use maple::element::*;
        let mut state = self.state.tracker();

        let mut layers = Layers::<Message, &'static str>::new(state.get("layers"));

        layers = layers.push(
            "w1",
            Window::new(
                state.get("window"),
                Row::new()
                    .push(Text::borrowed("Counter window").class("title"))
                    .push(Space)
                    .push(Space.class("close"))
                    .class("title"),
                Scroll::new(
                    state.get("scroll"),
                    Column::new()
                        .push(Button::new(state.get("up"), Text::borrowed("Up")).on_clicked(Message::UpPressed))
                        .push(Text::owned(format!("Hello {}! Count: {}", self.name, self.value)))
                        .push(Button::new(state.get("down"), Text::borrowed("Down")).on_clicked(Message::DownPressed))
                        .push(Input::new(state.get("name"), "username", Message::NameChanged))
                        .push(Input::password(
                            state.get("password"),
                            "password",
                            Message::PasswordChanged,
                        )),
                ),
            ),
        );

        layers = layers.push(
            "w2",
            Window::new(
                state.get("w2"),
                Row::new()
                    .push(Text::borrowed("Dummy window").class("title"))
                    .push(Space)
                    .push(Space.class("close"))
                    .class("title"),
                Space.class("dummy"),
            )
        );

        layers.into_node()
    }
}

fn main() {
    framework::run_model(Counter {
        value: 0,
        name: String::new(),
        password: String::new(),
        state: ManagedState::default(),
    }, "test_style.mss");
}