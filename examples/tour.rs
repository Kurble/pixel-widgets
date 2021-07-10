use pixel_widgets::event::Key;
use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::menu::MenuItem;

struct Tour {
    //
}

struct TourState {
    pub show_dummy: bool,
    pub show_login: bool,
    pub name: String,
    pub password: String,
    context_position: Option<(f32, f32)>,
}

enum Message {
    LoginPressed,
    ShowContext(f32, f32),
    CloseContext,
    ShowDummy(bool),
    ShowLogin(bool),
    NameChanged(String),
    PasswordChanged(String),
    PlanetSelected(&'static str),
}

impl Component for Tour {
    type Message = Message;
    type State = TourState;
    type Output = ();

    fn mount(&self) -> Self::State {
        TourState {
            show_dummy: false,
            show_login: false,
            name: "example".to_string(),
            password: "password".to_string(),
            context_position: None,
        }
    }

    fn view<'a>(&'a self, state: &'a TourState) -> Node<'a, Message> {
        let background = Column::new()
            .push(Button::new("Menu").on_clicked(Message::ShowContext(0.0, 32.0)))
            .push(Space)
            .push(
                Row::new()
                    .push(Space)
                    .push(Button::new("Open dummy").on_clicked(Message::ShowDummy(!state.show_dummy)))
                    .push(Button::new("Open login").on_clicked(Message::ShowLogin(!state.show_login))),
            );
        //.on_event(NodeEvent::MouseClick(Key::RightMouseButton), |ctx| {
        //    ctx.push(Message::ShowContext(ctx.cursor().0, ctx.cursor().1))
        //});

        let mut layers = Layers::<Message, &'static str>::with_background(background);

        let options = [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
        ];

        if let Some((x, y)) = state.context_position {
            layers = layers.push(
                "menu",
                Menu::new(x, y, Message::CloseContext)
                    .push(MenuItem::item("Open Dummy", Message::ShowDummy(!state.show_dummy)))
                    .push(MenuItem::item("Open Login", Message::ShowLogin(!state.show_login)))
                    .push(
                        MenuItem::menu("Planets").extend(
                            options
                                .iter()
                                .map(|&option| MenuItem::item(option, Message::PlanetSelected(option))),
                        ),
                    )
                    .push(MenuItem::item("Option D", None)),
            );
        }

        if state.show_dummy {
            layers = layers.push(
                "dummy_window",
                Window::new(
                    Row::new()
                        .push("Dummy window".with_class("title"))
                        .push(Space)
                        .push(Space.with_class("close"))
                        .with_class("title"),
                    Column::new().push("Select a planet from the dropdown list: ").push(
                        Dropdown::new().extend(options.iter().map(|&option| (option, Message::PlanetSelected(option)))),
                    ),
                ),
            );
        }

        if state.show_login {
            layers = layers.push(
                "login_window",
                Window::new(
                    Row::new()
                        .push("Login window".with_class("title"))
                        .push(Space)
                        .push(Space.with_class("close"))
                        .with_class("title"),
                    Scroll::new(
                        Column::new()
                            .push(
                                Input::new("username", state.name.as_str(), Message::NameChanged)
                                    .with_trigger_key(Key::Enter),
                            )
                            .push(Input::password(
                                "password",
                                state.password.as_str(),
                                Message::PasswordChanged,
                            ))
                            .push(Button::new("Login").on_clicked(Message::LoginPressed)),
                    ),
                ),
            );
        }

        layers.into_node()
    }

    fn update(&self, message: Self::Message, state: &mut TourState, _: &mut Runtime<Message>) -> Vec<()> {
        match message {
            Message::PlanetSelected(planet) => {
                println!("{} selected from the planets", planet);
            }
            Message::ShowDummy(show) => {
                state.show_dummy = show;
            }
            Message::ShowLogin(show) => {
                state.show_login = show;
            }
            Message::ShowContext(x, y) => {
                state.context_position = Some((x, y));
            }
            Message::CloseContext => {
                state.context_position = None;
            }
            Message::LoginPressed => {
                println!("login pressed!");
            }
            Message::NameChanged(name) => {
                state.name = name;
            }
            Message::PasswordChanged(password) => {
                state.password = password;
            }
        }

        Vec::new()
    }
}

#[tokio::main]
async fn main() {
    let window = winit::window::WindowBuilder::new()
        .with_title("Tour")
        .with_inner_size(winit::dpi::LogicalSize::new(960, 480));

    let mut sandbox = Sandbox::new(Tour {}, window).await;
    sandbox.ui.set_style(Style::from_file("examples/tour.pwss").unwrap());

    sandbox.run().await;
}
