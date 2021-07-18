use winit::window::WindowBuilder;

use pixel_widgets::declare_view;
use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;

struct Counter;

#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

impl Component for Counter {
    type Message = Message;
    type State = i32;
    type Output = ();

    fn mount(&self) -> Self::State {
        15
    }

    fn view(&self, state: &i32) -> Node<Message> {
        declare_view! {
            Column => {
                Button [text="Up", on_clicked=Message::UpPressed],
                Text [val=format!("Count: {}", *state)],
                Button [text="Down", on_clicked=Message::DownPressed]
            }
        }
    }

    fn update(
        &self,
        message: Self::Message,
        state: &mut i32,
        _runtime: &mut Runtime<Self::Message>,
    ) -> Vec<Self::Output> {
        match message {
            Message::UpPressed => {
                *state += 1;
                Vec::new()
            }
            Message::DownPressed => {
                *state -= 1;
                Vec::new()
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    let mut sandbox = Sandbox::new(Counter, window).await;
    sandbox.ui.set_style(Style::from_file("examples/counter.pwss").unwrap());

    sandbox.run().await;
}
