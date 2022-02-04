# The `view!` macro
The [`view!`](#macro.view) macro allows you to design interactive user interfaces easily, using a declarative syntax.
At it's core, the macro allow you to define a single widget and it's children. 

## A simple view
Each widget is declared using it's type. The macro will then use the `Default` implementation of the widget to construct it. Optionally you can add some properties to the declaration, as if you were filling in a struct. Finally, you have the option to add child widgets to the declaration in between `=> {}` braces.

Let's take a look at an example.
```rust
use pixel_widgets::prelude::*;

fn view<'a>() -> Node<'a, ()> {
    view! {
        Column => {
            Text { val: "Hello world" },
            Button { text: "Click me" },
        }
    }
}
```
In this example, a column widget is declared with two children. The first child is a `Text` widget, with it's value set to `"Hello world"`. The second child is a `Button` with it's text set to `"Click me"`.
Setting the value of a `Text` in this case, happens by assigning a `&str` to the `val` property. All of the widgets have many different properties that you can set, and you can find them by viewing the documention of the widget. In rust code, properties are implemented as methods on the widget that follow the builder pattern; They take the widget by value, take a single argument, and return `Self`. Like so:
```rust
pub struct Text {
    text: String,
}

impl Text {
    /// Sets the text value.
    pub fn val(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
}
```

### Built-in properties
Some properties are provided by the implementation of `Node`, and must be the last property in your list in order for your other properties to be available. Specifically, these are the `key` and `class` properties.

The `key` property is used to set a custom key to the node, which is used by the runtime to identify what state was associated with it after the view was updated. It is useful to set some unique key when you have widgets of the same type, and a new one is inserted or removed in the middle.

The `class` property is used to select rules from the style engine, like you would in css. Unlike css, pixel-widgets does not allow for an `id`, as you don't have access to "the dom", and classes serve the same purpose anyway.

## Conditional rendering
While the previous example is already pretty useful when declaring a user interface component, you typically want to turn some parts of your user interface on and off based on the state. Pixel widgets declarative syntax supports if statements for this reason. 
```rust
use pixel_widgets::prelude::*;

struct State {
    show_secret: bool,
    foo: bool,
    bar: bool,
}

fn view<'a>(state: &'a State) -> Node<'a, ()> {
    view! {
        Column => {
            Text { val: "Hello world" },

            [if state.show_secret]
            Text { val: "Secret message" },

            [if state.foo] 
            Text { val: "foo" },
            [else if state.bar] 
            Text { val: "bar" },
            [else]
            Text { val: "foobar"},
        }
    }
}
```
Note that these statements only support single widgets, and no groups. This is unfortunately a limitation of the way the macro works. If you would like to conditionally render multiple widgets, you should wrap them in a layout, like so:
```rust
use pixel_widgets::prelude::*;

fn view<'a>() -> Node<'a, ()> {
    view! {
        Column => {
            Text { val: "Title" },
            [if 2 > 1]
            Column => {
                Text { val: "Line 1" },
                Text { val: "Line 2" },
                Text { val: "Line 3" },
            }
        }
    }
}
```

## Iteration
If you are making a list of items, or maybe populating a dropdown, for loops can come in really handy. This example shows how to popuplate a dropdown box using a declarative for loop.
```rust
use pixel_widgets::prelude::*;

enum Msg {
    Selected(usize),
}

fn view<'a>() -> Node<'a, Msg> {
    let options = ["Option A", "Option B", "Option C"];

    view! {
        Dropdown { on_select: Msg::Selected } => {
            [for option in options]
            Text { val: option },
        }
    }
}
```
Like if/else statements, for loops also produce one widget per iteration. The solution is much the same: you should wrap groups of widget in a layout if you need it.

## Match
It's also possible to match an expression. The syntax is a bit more awkward due to the macro limits.
```rust
use pixel_widgets::prelude::*;

fn view<'a>() -> Node<'a, ()> {
    let x = 2;
    view! {
        Column => {
            [match x]
            [case 1] 
            Text { val: "Option 1" },
            [case 2] 
            Text { val: "Option 2" },
            [case 3] 
            Text { val: "Option 3" },
            [case 4] 
            Text { val: "Option 4" },
            [case _] 
            Text { val: "Unknown option" },
        }
    }
}
```

## Integrating your components
Not just widgets can be used in declarative syntax. In fact, any type that implements `Default` and has a `into_node()` method can be used. this means you can compose complex user interfaces from components. By default, the `Component` trait already defines an `into_node()` method for you. The only thing left to do is to sure your component implements `Default` and has some builder methods if you need to set any properties on your component.

### Properties
You see, the properties that we have been using through this guide work by calling methods on the default constructed widgets. For a property to work, you must make a method that takes `self` and one argument. It also has to return `Self`. You can then use the method as a property in declarative syntax.
