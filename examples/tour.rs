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
    PlanetSelected(usize),
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
        let options = [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
        ];

        declare_view! {
            Layers => {
                Column => {
                    Button [text="Menu", on_clicked=Message::ShowContext(0.0, 32.0)],
                    Space,
                    Row => {
                        Space,
                        Button [text="Open dummy", on_clicked=Message::ShowDummy(!state.show_dummy)],
                        Button [text="Open login", on_clicked=Message::ShowLogin(!state.show_login)]
                    }
                },

                :if let Some(pos) = state.context_position => Menu [
                    position=pos,
                    on_close=Message::CloseContext,
                    items=vec![
                        MenuItem::item("Open Dummy", Message::ShowDummy(!state.show_dummy)),
                        MenuItem::item("Open Login", Message::ShowLogin(!state.show_login)),
                        MenuItem::menu("Planets").extend(
                            options
                                .iter()
                                .map(|&option| MenuItem::item(option, Message::PlanetSelected(0))),
                        ),
                        MenuItem::item("Option D", None)
                    ],
                    key = 0
                ],

                :if state.show_dummy => Window [key=1] => {
                    Row [class="title"] => {
                        Text [val="Dummy window", class="title"],
                        Space,
                        Space [class="close"]
                    },
                    Column => {
                        Text [val="Select a planet from the dropdown list: "],
                        Dropdown [on_select=Message::PlanetSelected] => {
                            :for &option in options.iter() => Text [val=option]
                        }
                    }
                },

                :if state.show_login => Window [key=2] => {
                    Row [class="title"] => {
                        Text [val="Login window", class="title"],
                        Space,
                        Space [class="close"]
                    },
                    Column => {
                        Input [
                            placeholder="username",
                            val=state.name.as_str(),
                            on_change=Message::NameChanged,
                            trigger_key=Key::Enter
                        ],
                        Input [
                            placeholder="password",
                            val=state.password.as_str(),
                            on_change=Message::PasswordChanged,
                            password=true
                        ],
                        Button [text="Login", on_clicked=Message::LoginPressed]
                    }
                }
            }
        }
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
