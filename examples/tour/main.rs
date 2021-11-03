use pixel_widgets::event::Key;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::menu::MenuItem;
use winit::window::WindowBuilder;

#[derive(Default)]
struct Tour;

struct TourState {
    pub show_dummy: bool,
    pub show_login: bool,
    context_position: Option<(f32, f32)>,
}

mod dummy_window;
mod login_window;

use dummy_window::DummyWindow;
use login_window::LoginWindow;

pub enum Message {
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

        view! {
            Layers => {
                Column => {
                    Button { text: "Menu", on_clicked: Message::ShowContext(0.0, 32.0) }
                    Spacer
                    Row => {
                        Spacer
                        Button { text: "Open dummy", on_clicked: Message::ShowDummy(!state.show_dummy) }
                        Button { text: "Open login", on_clicked: Message::ShowLogin(!state.show_login) }
                    }
                }

                :if let Some(pos) = state.context_position => Menu {
                    position: pos,
                    on_close: Message::CloseContext,
                    items: vec![
                        MenuItem::item("Open Dummy", Message::ShowDummy(!state.show_dummy)),
                        MenuItem::item("Open Login", Message::ShowLogin(!state.show_login)),
                        MenuItem::menu("Planets").extend(
                            options
                                .iter()
                                .map(|&option| MenuItem::item(option, Message::PlanetSelected(0))),
                        ),
                        MenuItem::item("Option D", None)
                    ],
                    key: 0
                }

                :if state.show_dummy => DummyWindow,

                :if state.show_login => LoginWindow,
            }
        }
    }

    fn update(&self, message: Self::Message, mut state: State<TourState>, _: Context<Message, ()>) {
        match message {
            Message::ShowDummy(show) => state.show_dummy = show,
            Message::ShowLogin(show) => state.show_login = show,
            Message::ShowContext(x, y) => state.context_position = Some((x, y)),
            Message::CloseContext => state.context_position = None,
            _ => (),
        }
    }
}

#[tokio::main]
async fn main() {
    Sandbox::new(
        Tour,
        Style::builder()
            .rule(RuleBuilder::new("row").fill_width().padding_all(2.0))
            .component::<DummyWindow>()
            .component::<LoginWindow>(),
        WindowBuilder::new()
            .with_title("Tour")
            .with_inner_size(winit::dpi::LogicalSize::new(960, 480)),
    )
    .await
    .run()
    .await;
}
