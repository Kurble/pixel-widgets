use pixel_widgets::event::Key;
use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::menu::MenuItem;

struct Tour;

struct TourState {
    pub show_dummy: bool,
    pub show_login: bool,
    context_position: Option<(f32, f32)>,
}

#[derive(Default)]
struct LoginWindow;

struct LoginWindowState {
    pub name: String,
    pub password: String,
}

#[derive(Default)]
struct DummyWindow;

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

                :if state.show_dummy => DummyWindow,

                :if state.show_login => LoginWindow
            }
        }
    }

    fn update(&self, message: Self::Message, state: &mut TourState, _: &mut Runtime<Message>, _: &mut Context<()>) {
        match message {
            Message::ShowDummy(show) => state.show_dummy = show,
            Message::ShowLogin(show) => state.show_login = show,
            Message::ShowContext(x, y) => state.context_position = Some((x, y)),
            Message::CloseContext => state.context_position = None,
            _ => (),
        }
    }
}

impl Component for LoginWindow {
    type State = LoginWindowState;
    type Message = Message;
    type Output = Message;

    fn mount(&self) -> LoginWindowState {
        LoginWindowState {
            name: "example".to_string(),
            password: "password".to_string(),
        }
    }

    fn view<'a>(&'a self, state: &'a LoginWindowState) -> Node<'a, Message> {
        declare_view!{
            Window => {
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

    fn update(&self, message: Message, state: &mut LoginWindowState, _: &mut Runtime<Message>, _: &mut Context<Message>) {
        match message {
            Message::NameChanged(name) => state.name = name,
            Message::PasswordChanged(password) => state.password = password,
            Message::LoginPressed => println!("login pressed!"),
            _ => (),
        }
    }
}

impl<'a> IntoNode<'a, Message> for LoginWindow {
    fn into_node(self) -> Node<'a, Message> {
        Node::from_component(self)
    }
}

impl Component for DummyWindow {
    type State = ();
    type Message = Message;
    type Output = Message;

    fn mount(&self) -> Self::State {
        ()
    }

    fn view<'a>(&'a self, _: &'a Self::State) -> Node<'a, Self::Message> {
        let options = [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
        ];

        declare_view!{
            Window => {
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
            }
        }
    }

    fn update(&self, message: Message, _: &mut (), _: &mut Runtime<Message>, _: &mut Context<Message>) {
        if let Message::PlanetSelected(planet) = message {
            println!("{} selected from the planets", planet);
        }
    }
}

impl<'a> IntoNode<'a, Message> for DummyWindow {
    fn into_node(self) -> Node<'a, Message> {
        Node::from_component(self)
    }
}

#[tokio::main]
async fn main() {
    let window = winit::window::WindowBuilder::new()
        .with_title("Tour")
        .with_inner_size(winit::dpi::LogicalSize::new(960, 480));

    let mut sandbox = Sandbox::new(Tour, window).await;
    sandbox.ui.set_style(Style::from_file("examples/tour.pwss").unwrap());

    sandbox.run().await;
}
