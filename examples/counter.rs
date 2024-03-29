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
    type State = i32;
    type Message = Message;
    type Output = ();

    // Creates the state of our component when it's first constructed.
    fn mount(&self, _: &mut Runtime<Message>) -> Self::State {
        self.initial_value
    }

    // Generates the widgets for this component, based on the current state.
    fn view(&self, state: &i32) -> Node<Message> {
        // You can build the view using declarative syntax with the view! macro,
        //  but you can also construct widgets using normal rust code.
        view! {
            Column => {
                Button { text: "Up", on_clicked: Message::UpPressed },
                Text { val: format!("Count: {}", *state) },
                Button { text: "Down", on_clicked: Message::DownPressed },
            }
        }
    }

    // Updates the component state based on a message.
    fn update(&self, message: Message, mut state: DetectMut<i32>, _: &mut Runtime<Message>, _: &mut Context<()>) {
        match message {
            Message::UpPressed => *state += 1,
            Message::DownPressed => *state -= 1,
        }
    }
}

#[tokio::main]
async fn main() {
    Sandbox::new(
        Counter { initial_value: 15 },
        StyleBuilder::default(),
        WindowBuilder::new()
            .with_title("Counter")
            .with_inner_size(winit::dpi::LogicalSize::new(240, 240)),
    )
    .await
    .unwrap()
    .run()
    .await;
}
