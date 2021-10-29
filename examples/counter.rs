use winit::window::WindowBuilder;

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
        _context: &mut Context<()>,
    ) {
        match message {
            Message::UpPressed => {
                *state += 1;
            }
            Message::DownPressed => {
                *state -= 1;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));
    Sandbox::new(Counter, window).await.run().await;
}
