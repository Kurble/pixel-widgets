use image::io::Reader as ImageReader;
use pixel_widgets::event::Key;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::menu::MenuItem;
use winit::window::WindowBuilder;

#[derive(Default)]
struct Tour;

struct State {
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
    type State = State;
    type Output = ();

    fn mount(&self, _: &mut Runtime<Message>) -> Self::State {
        State {
            show_dummy: false,
            show_login: false,
            context_position: None,
        }
    }

    fn view<'a>(&'a self, state: &'a State) -> Node<'a, Message> {
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

                [if let Some(pos) = state.context_position]
                Menu {
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

                [if state.show_dummy]
                DummyWindow,

                [if state.show_login]
                LoginWindow,
            }
        }
    }

    fn update(&self, message: Message, mut state: DetectMut<State>, _: &mut Runtime<Message>, _: &mut Context<()>) {
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
    let mut style = Style::builder();

    let button = style.load_patch("button.9.png", || {
        Ok(ImageReader::open("examples/button.9.png")?.decode()?.into_rgba8())
    });
    let button_hover = style.load_patch("button.hover.9.png", || {
        Ok(ImageReader::open("examples/button.hover.9.png")?.decode()?.into_rgba8())
    });
    let button_pressed = style.load_patch("button.pressed.9.png", || {
        Ok(ImageReader::open("examples/button.pressed.9.png")?
            .decode()?
            .into_rgba8())
    });

    let dropdown = style.load_patch("dropdown.9.png", || {
        Ok(ImageReader::open("examples/dropdown.9.png")?.decode()?.into_rgba8())
    });
    let dropdown_hover = style.load_patch("dropdown.hover.9.png", || {
        Ok(ImageReader::open("examples/dropdown.hover.9.png")?
            .decode()?
            .into_rgba8())
    });
    let dropdown_open = style.load_patch("dropdown.open.9.png", || {
        Ok(ImageReader::open("examples/dropdown.open.9.png")?
            .decode()?
            .into_rgba8())
    });

    let input = style.load_patch("input.9.png", || {
        Ok(ImageReader::open("examples/input.9.png")?.decode()?.into_rgba8())
    });
    let input_focused = style.load_patch("input.focused.9.png", || {
        Ok(ImageReader::open("examples/input.focused.9.png")?
            .decode()?
            .into_rgba8())
    });

    Sandbox::new(
        Tour,
        style
            .rule(RuleBuilder::new("row").fill_width().padding_all(2.0))
            .rule(RuleBuilder::new("layers > *").background_color(Color::rgb(0.6, 0.6, 0.6)))
            .rule(RuleBuilder::new("button").background_patch(button, Color::white()))
            .rule(RuleBuilder::new("button text").color(Color::black()))
            .rule(RuleBuilder::new("button:hover").background_patch(button_hover, Color::white()))
            .rule(RuleBuilder::new("button:pressed").background_patch(button_pressed, Color::white()))
            .rule(RuleBuilder::new("button:pressed text").color(Color::white()))
            .rule(
                RuleBuilder::new("dropdown")
                    .background_patch(dropdown, Color::white())
                    .color(Color::rgb(0.6, 0.6, 1.0)),
            )
            .rule(RuleBuilder::new("dropdown:hover").background_patch(dropdown_hover, Color::white()))
            .rule(RuleBuilder::new("dropdown:open").background_patch(dropdown_open, Color::white()))
            .rule(RuleBuilder::new("dropdown text").color(Color::black()))
            .rule(RuleBuilder::new("input").background_patch(input, Color::white()))
            .rule(RuleBuilder::new("input:focused").background_patch(input_focused, Color::white()))
            .component::<DummyWindow>()
            .component::<LoginWindow>(),
        WindowBuilder::new()
            .with_title("Tour")
            .with_inner_size(winit::dpi::LogicalSize::new(960, 480)),
    )
    .await
    .unwrap()
    .run()
    .await;
}
