# Pixel-widgets
[![Documentation](https://docs.rs/pixel-widgets/badge.svg)](https://docs.rs/pixel-widgets)
[![Crates.io](https://img.shields.io/crates/v/pixel-widgets.svg)](https://crates.io/crates/pixel-widgets)
![License](https://img.shields.io/crates/l/pixel-widgets.svg)

pixel-widgets is a component based user interface library focused on integratability in graphical applications.

# Features
- Very compact and easy API
- API agnostic rendering
- [wgpu](https://github.com/gfx-rs/wgpu-rs) based renderer included
- Styling using [stylesheets](stylesheet/index.html)
- Built in [widgets](widget/index.html)

# Overview
User interfaces in pixel-widgets are composed of [`Component`](trait.Component.html)s. These components manage their own state, and generate ui elements when that state is mutated. Each component implements some methods:
- [`view`](trait.Component.html#tymethod.view) - this method renders the ui elements for the current component state. When the state is updated, the view will be rendered again.
method:
- [`update`](trait.Component.html#tymethod.update) - ui elements generate messages that will be passed to the update method. In here, a component will update it's internal state based on these messages.

# Quick start
This example shows how to define a component and run it in the included sandbox. More work is required if you want to use pixel-widgets in your own game engine.
```rust
use pixel_widgets::prelude::*;

// The main component for our simple application
struct Counter {
    initial_value: i32,
}
```

Then, we have to define a message type. The message type should be able to tell us what happend in the ui.
```rust
// The message type that will be used in our `Counter` component.
#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}
```

And finally, we must implement [`Component`](component/trait.Component.html)
```rust no_run
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
    fn mount(&self) -> Self::State {
        self.initial_value
    }

    // Generates the widgets for this component, based on the current state.
    fn view<'a>(&'a self, state: &'a i32) -> Node<'a, Message> {
        // You can build the view using declarative syntax with the view! macro,
        //  but you can also construct widgets using normal rust code.
        view! {
            Column => {
                Button { text: "Up", on_clicked: Message::UpPressed }
                Text { val: format!("Count: {}", *state) }
                Button { text: "Down", on_clicked: Message::DownPressed }
            }
        }
    }

    // Updates the component state based on a message.
    fn update(&self, message: Message, mut state: State<i32>, _context: Context<Message, ()>) {
        match message {
            Message::UpPressed => *state += 1,
            Message::DownPressed => *state -= 1,
        }
    }
}

#[tokio::main]
async fn main() {
    let window = winit::window::WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));
    Sandbox::new(Counter { initial_value: 15 }, window).await.run().await;
}
```
# Examples
If you want more [examples](https://github.com/Kurble/pixel-widgets/tree/master/examples), check out the examples directory in the git repository.
