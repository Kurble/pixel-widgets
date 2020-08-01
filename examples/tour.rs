use pixel_widgets::prelude::*;
use pixel_widgets::Command;

struct Tour {
    pub show_dummy: bool,
    pub show_login: bool,
    pub name: String,
    pub password: String,
    pub state: ManagedState<String>,
}

enum Message {
    LoginPressed,
    ShowDummy,
    ShowLogin,
    NameChanged(String),
    PasswordChanged(String),
    PlanetSelected(&'static str),
}

impl Model for Tour {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Vec<Command<Message>> {
        match message {
            Message::PlanetSelected(planet) => {
                println!("{} selected from the planets", planet);
            }
            Message::ShowDummy => {
                self.show_dummy = true;
            }
            Message::ShowLogin => {
                self.show_login = true;
            }
            Message::LoginPressed => {
                println!("login pressed!");
            }
            Message::NameChanged(name) => {
                self.name = name;
            }
            Message::PasswordChanged(password) => {
                self.password = password;
            }
        }

        Vec::new()
    }

    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();

        let background = Column::new()
            .push(Space)
            .push(Row::new()
                .push(Space)
                .push(Button::new(state.get("dummy"), Text::new("Open dummy")).on_clicked(Message::ShowDummy))
                .push(Button::new(state.get("login"), Text::new("Open login")).on_clicked(Message::ShowLogin))
            );

        let mut layers = Layers::<Message, &'static str>::with_background(state.get("layers"), background);

        let options = ["Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto"];

        if self.show_dummy {
            layers = layers.push(
                "dummy_window",
                Window::new(
                    state.get("dummy_window"),
                    Row::new()
                        .push(Text::new("Dummy window").class("title"))
                        .push(Space)
                        .push(Space.class("close"))
                        .class("title"),
                    Column::new()
                        .push(Text::new("Select a planet from the dropdown list: "))
                        .push(Dropdown::new(state.get("dd"))
                            .extend(options.iter().map(|&option| (Text::new(option), Message::PlanetSelected(option))))
                        ),
                ),
            );
        }

        if self.show_login {
            layers = layers.push(
                "login_window",
                Window::new(
                    state.get("login_window"),
                    Row::new()
                        .push(Text::new("Login window").class("title"))
                        .push(Space)
                        .push(Space.class("close"))
                        .class("title"),
                    Scroll::new(
                        state.get("scroll"),
                        Column::new()
                            .push(Input::new(state.get("name"), "username", Message::NameChanged))
                            .push(Input::password(
                                state.get("password"),
                                "password",
                                Message::PasswordChanged,
                            ))
                            .push(Button::new(state.get("login"), Text::new("Login")).on_clicked(Message::LoginPressed)),
                    ),
                ),
            );
        }

        layers.into_node()
    }
}

fn main() {
    let model = Tour {
        show_dummy: false,
        show_login: false,
        name: String::new(),
        password: String::new(),
        state: ManagedState::default(),
    };

    let window = winit::window::WindowBuilder::new()
        .with_title("Tour")
        .with_inner_size(winit::dpi::LogicalSize::new(960, 480));

    pixel_widgets::sandbox::run(model, std::path::PathBuf::from("./examples"), "tour.pwss", window);
}
