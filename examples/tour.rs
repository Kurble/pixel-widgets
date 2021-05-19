use pixel_widgets::event::{Key, NodeEvent};
use pixel_widgets::prelude::*;
use pixel_widgets::widget::menu::MenuItem;
use pixel_widgets::Command;

struct Tour {
    pub show_dummy: bool,
    pub show_login: bool,
    pub name: String,
    pub password: String,
    pub context: pixel_widgets::widget::menu::State,
    pub state: ManagedState<String>,
}

enum Message {
    LoginPressed,
    ShowContext(f32, f32),
    ShowDummy,
    ShowLogin,
    NameChanged(String),
    PasswordChanged(String),
    SlideChanged(f32),
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
            Message::ShowContext(x, y) => {
                self.context.open(x, y);
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
            Message::SlideChanged(x) => {
                println!("slide to {}", x);
            }
        }

        Vec::new()
    }

    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();

        let background = Column::new()
            .push(Space)
            .push(
                Row::new()
                    .push(Space)
                    .push(Button::new(state.get("dummy"), Text::new("Open dummy")).on_clicked(Message::ShowDummy))
                    .push(Button::new(state.get("login"), Text::new("Open login")).on_clicked(Message::ShowLogin)),
            )
            .on_event(NodeEvent::MouseClick(Key::RightMouseButton), |ctx| {
                ctx.push(Message::ShowContext(ctx.cursor().0, ctx.cursor().1))
            });

        let mut layers = Layers::<Message, &'static str>::with_background(state.get("layers"), background);

        let options = [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
        ];

        layers = layers.push(
            "menu",
            Menu::new(&mut self.context).extend(vec![
                MenuItem::Item {
                    content: Text::new("Open Dummy").into_node(),
                    on_select: Some(Message::ShowDummy),
                },
                MenuItem::Item {
                    content: Text::new("Open Login").into_node(),
                    on_select: Some(Message::ShowLogin),
                },
                MenuItem::Menu {
                    content: Text::new("Planets").into_node(),
                    items: options
                        .iter()
                        .map(|option| MenuItem::Item {
                            content: Text::new(option.to_string()).into_node(),
                            on_select: Some(Message::PlanetSelected(option)),
                        })
                        .collect(),
                },
                MenuItem::Item {
                    content: Text::new("Option D").into_node(),
                    on_select: None,
                },
            ]),
        );

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
                        .push(Slider::new(state.get("slider"), 0.0, 100.0, Message::SlideChanged))
                        .push(Text::new("Select a planet from the dropdown list: "))
                        .push(
                            Dropdown::new(state.get("dd")).extend(
                                options
                                    .iter()
                                    .map(|&option| (Text::new(option), Message::PlanetSelected(option))),
                            ),
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
                            .push(
                                Input::new(state.get("name"), "username", Message::NameChanged)
                                    .with_trigger_key(Key::Enter),
                            )
                            .push(Input::password(
                                state.get("password"),
                                "password",
                                Message::PasswordChanged,
                            ))
                            .push(
                                Button::new(state.get("login"), Text::new("Login")).on_clicked(Message::LoginPressed),
                            ),
                    ),
                ),
            );
        }

        layers.into_node()
    }
}

#[tokio::main]
async fn main() {
    let model = Tour {
        show_dummy: false,
        show_login: false,
        name: String::new(),
        password: String::new(),
        context: Default::default(),
        state: ManagedState::default(),
    };

    let window = winit::window::WindowBuilder::new()
        .with_title("Tour")
        .with_inner_size(winit::dpi::LogicalSize::new(960, 480));

    let loader = pixel_widgets::loader::FsLoader::new("./examples".into()).unwrap();

    let mut sandbox = Sandbox::new(model, loader, window).await;

    sandbox
        .ui
        .set_stylesheet("tour.pwss")
        .await
        .expect("Unable to load stylesheet");

    sandbox.run().await;
}
