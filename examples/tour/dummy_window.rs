use super::*;

#[derive(Default)]
pub struct DummyWindow;

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

        declare_view! {
            Window => {
                Row [class="title"] => {
                    Text [val="Dummy window", class="title"],
                    Spacer,
                    Spacer [class="close"]
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