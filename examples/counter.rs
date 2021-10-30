use winit::window::WindowBuilder;

use pixel_widgets::prelude::*;

// The main component for our simple application
#[derive(Default)]
struct Counter {
    initial_value: i32,
}

// The message type that will be used in our `Counter` component.
#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

impl Component for Counter {
    type Message = Message;
    type State = i32;
    type Output = ();

    // Creates the state of our component when it's first constructed.
    fn mount(&self) -> Self::State {
        self.initial_value
    }

    // Generates the widgets for this component, based on the current state.
    fn view(&self, state: &i32) -> Node<Message> {
        // You can build the view using declarative syntax in the declare_view! macro,
        //  but you can also construct widgets using normal rust code.
        view! {
            Column => {
                Button [text="Up", on_clicked=Message::UpPressed],
                Text [val=format!("Count: {}", *state)],
                Button [text="Down", on_clicked=Message::DownPressed]
            }
        }
    }

    // Updates the component state based on a message.
    fn update(&self, message: Self::Message, state: &mut i32, _context: Context<Message, ()>) {
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
    let component = Counter { initial_value: 15 };
    Sandbox::new(component, window).await.run().await;
}
