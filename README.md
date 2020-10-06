# Pixel-widgets
[![Documentation](https://docs.rs/pixel-widgets/badge.svg)](https://docs.rs/pixel-widgets)
[![Crates.io](https://img.shields.io/crates/v/pixel-widgets.svg)](https://crates.io/crates/pixel-widgets)
![License](https://img.shields.io/crates/l/pixel-widgets.svg)

pixel-widgets is a user interface library focused on use for games. It's architecture is inspired by elm, 
since it's very fitting for rust mutability patterns.

# Features
- Very compact and easy API
- Render agnostic rendering
- [wgpu](https://github.com/gfx-rs/wgpu-rs) based renderer included
- Styling using [stylesheets](stylesheet/index.html)
- Built in [widgets](widget/index.html)

Check out the [examples](https://github.com/Kurble/pixel-widgets/tree/master/examples) to get started quickly.

# Overview
User interfaces in pixel-widgets are all defined by implementing a [`Model`](trait.Model.html), serving as the data model
for your user interface. The model then has to implement some methods:
- [`view`](trait.Model.html#tymethod.view) - for generating a tree of ui widgets. These are retained for as long as
the model is not mutated. Ui widgets generate _messages_ when they are interacted with, which leads us to the next
method:
- [`update`](trait.Model.html#tymethod.update) - modifies the model based on a message that was generated
by the view

Other ways of updating the ui, such as futures and subscriptions will be be coming in the future.

# Quick start
Setting up a ui with pixel-widgets is easy. You start with defining a model.
```rust
use pixel_widgets::prelude::*;

pub struct Counter {
    // a state manager, used for remembering the state of our buttons
    state: ManagedState<String>,
    // the counter value
    count: i32,
}
```

Then, we have to define a message type. The message type should be able to tell us what happend in the ui.
```rust
pub enum Message {
   UpClicked,
   DownClicked,
}
```

And finally, we must implement [`Model`](trait.Model.html) on our state
```rust
use pixel_widgets::prelude::*;

pub struct Counter {
   state: ManagedState<String>,
   count: i32,
}

pub enum Message {
   UpClicked,
   DownClicked,
}

impl Model for Counter {
    // define our message type
    type Message = Message;

    fn update(&mut self, message: Message) -> Vec<Command<Message>> {
        match message {
            Message::UpClicked => self.count += 1,
            Message::DownClicked => self.count -= 1,
        }
    }

    // Note that the view is allowed to keep mutable references to the model.
    // As soon as the model is accessed mutably, the `Ui` will destroy the existing view.
    fn view(&mut self) -> Node<Message> {
        let mut state = self.state.tracker();
        Column::new()
            .push(Button::new(state.get("up"), Text::new("Up"))
                .on_clicked(Message::UpClicked)
            )
            .push(Text::new(format!("Count: {}", self.count)))
            .push(Button::new(state.get("down"), Text::new("Down"))
                .on_clicked(Message::DownClicked)
            )
            .into_node()
    }
}

// Now that we have a model that can be used with pixel-widgets,
// we can pass it to the sandbox to quickly see some results!
#[tokio::main]
async fn main() {
    let model = Counter {
        state: ManagedState::default(),
        count: 0,
    };

    let window = winit::window::WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    let loader = pixel_widgets::loader::FsLoader::new(".".into()).unwrap();

    let mut sandbox = Sandbox::new(model, loader, window).await;
    sandbox.ui.set_stylesheet("counter.pwss").await.unwrap();
    sandbox.run().await;
}
```
