use super::*;

#[derive(Default)]
pub struct DummyWindow;

impl Component for DummyWindow {
    type State = ();
    type Message = Message;
    type Output = Message;

    fn mount(&self) {}

    fn view<'a>(&'a self, _: &'a Self::State) -> Node<'a, Self::Message> {
        let options = [
            "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
        ];

        view! {
            Window => {
                Row { class: "title" } => {
                    Text { val: "Dummy window", class: "title" }
                    Spacer
                    Spacer { class: "close" }
                }
                Column => {
                    Text { val: "Select a planet from the dropdown list: " }
                    Dropdown { on_select: Message::PlanetSelected } => {
                        :for &option in options.iter() => Text { val: option }
                    }
                }
            }
        }
    }

    fn style() -> StyleBuilder {
        StyleBuilder::default().rule(RuleBuilder::new("window").background_color(Color::rgb(0.5, 0.3, 0.3)))
    }

    fn update(&self, message: Message, _: State<()>, _: Context<Message, Message>) {
        if let Message::PlanetSelected(planet) = message {
            println!("{} selected from the planets", planet);
        }
    }
}
