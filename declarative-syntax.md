The [`declare_view!`](#macro.declare_view) macro allows you to design interactive user interfaces easily, using a declarative syntax.
At it's core, the macro allow you to define a single widget and it's children. 

## A simple view
Each widget is declared using it's type. The macro will then use the `Default` implementation of the widget to construct it. Optionally you can add some properties to the declaration, in between `[]` brackets. Finally, you have the option to add child widgets to the declaration in between `{}` braces.

Let's take a look at an example.
```rust
declare_view! {
    Column {
        Text [val = "Hello world"],
        Button [text = "Click me"]
    }
}
```
In this example, a column widget is declared with two children. The first child is a `Text` widget, with it's value set to `"Hello world"`. The second child is a `Button` with it's text set to `"Click me"`.

## Conditional rendering
While the previous example is already pretty useful when declaring a user interface component, you typically want to turn some parts of your user interface on and off based on the state. Pixel widgets declarative syntax supports if statements for this reason. 
```rust
declare_view! {
    Column {
        Text [val = "Hello world"],

        :if state.show_secret => Text [val = "Secret message"],

        :if state.foo => Text [val = "foo"],
        :else if state.bar => Text [val = "bar"],
        :else => Text [val = "foobar"],
    }
}
```

## Iteration
If you are making a list of items, or maybe populating a dropdown, for loops can come in really handy. This example shows how to popuplate a dropdown box using a declarative for loop.
```rust
let options = ["Option A", "Option B", "Option C"];

declare_view! {
    Dropdown => {
        :for option in options => Text [val=option]
    }
}
```

## Integrating your components
Not just widgets can be used in declarative syntax. In fact, any type that implements `Default` and has a `into_node()` method can be used. this means you can compose complex user interfaces from components. By default, the `Component` trait already defines an `into_node()` method for you. The only thing left to do is to sure your component implements `Default` and has some builder methods if you need to set any properties on your component.

### Properties
You see, the properties that we have been using through this guide work by calling methods on the default constructed widgets. For a property to work, you must make a method that takes `self` and one argument. It also has to return `Self`. You can then use the method as a property in declarative syntax.