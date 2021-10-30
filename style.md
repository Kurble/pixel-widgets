Style in pixel-widgets is defined using stylesheets. These stylesheets are loaded from a file, with a format that is a
syntactically a subset of css. The stylesheets are called `pwss` - *p*ixel-*w*idgets *s*tyle*s*heets.
# Features
- Select widgets, .classes and :states
- Select child widgets, sibling widgets
# Example
```ignore
column {
    align-horizontal: center;
}
button {
    background: #444;
    padding: 5;
}
button:hover {
    background: #666;
}
button:pressed {
    background: #222;
}
button:hover > text {
    color: #f00;
}
text {
    text-size: 24;
}
```
The example sets a few properties on some of the widgets. Just try it out with the examples in the example
directory and see for yourself what the effect is.
# Syntax
Each pwss file contains a collection of _rules_. Rules are a group of _declarations_ that are applied to _selected_
widgets.
## Rules
A selector has the following format:
```ignore
<selector> <selector> ... {
    <property>: <value>;
    <property>: <value>;
    ...
}
```
The first line expects some selectors. Class selectors can be differentiated
from widget selectors by adding a period in front, as in `.class`, and state selectors have a ':' in front.
```ignore
window column button {
    background: @button.png;
}
```
Entering multiple selectors like in this example will look for a `button` inside a `column` inside a `window`.
## Selectors
This table describes the supported selectors
| selector | example | description |
|---|---|---|
| `*` | `*` | selects all widgets |
| `widget` | `text` | selects all text widgets |
| `.class` | `.fancy` | selects all widgets that have the class "fancy" |
| `.. widget` | `.fancy text` | selects all text widgets that are a descendant of a "fancy" classed widget |
| `>widget` | `.fancy > text` | selects all text widgets that are a direct child of a "fancy" classed widget |
| `+widget` | `.fancy + text` | selects all widgets that follow directly after a "fancy" classed widget |
| `~widget` | `.fancy ~ text` | selects all widgets that follow after a "fancy" classed widget |
| `:state` | `button:hover` | selects all buttons that are hovered by the mouse |
| `:nth-child(n)` | `text:nth-child(2)` | selects text widgets that are the third child of their parent |
| `:nth-last-child(n)` | `text:nth-last-child(2)` | selects text widgets that are the third child of their parent, counted from the last widget |
| `:nth-child(odd)` | `text:nth-child(odd)` | selects text widgets that are an odd child of their parent |
| `:nth-child(even)` | `text:nth-child(even)` | selects text widgets that are an even child of their parent |
| `:not(selector)` | `button:not(:pressed)` | selects button widgets that are not pressed |
| `:only-child` | `column > *:only-child` | selects the only child of a column when the column has only one child |
## Properties
The interior of a rule consists of a number of declarations. These declarations are what specifies style.
A declaration starts with a property, and each property has it's own associated format.
Take a look at the table to see what properties exist.
| key | description | format |
|---|---|---|
| width | widget width | size |
| height | widget height | size |
| background | Background for the widget that full covers the layout rect | background |
| padding | Amount of padding to use on each side of the content | rectangle |
| padding-left | Amount of padding to use on the left side of the content | number |
| padding-right | Amount of padding to use on the right side of the content | number |
| padding-top | Amount of padding to use on the top side of the content | number |
| padding-bottom | Amount of padding to use on the bottom side of the content | number |
| margin | Amount of margin to use on each side of the widget | rectangle |
| margin-left | Amount of margin to use on the left side of the widget | number |
| margin-right | Amount of margin to use on the right side of the widget | number |
| margin-top | Amount of margin to use on the top side of the widget | number |
| margin-bottom | Amount of margin to use on the bottom side of the widget | number |
| font | Font to use for text rendering | url |
| color | Color to use for foreground drawing, including text | color |
| text_size | Size of text | number |
| text_wrap | Wrapping strategy for text | textwrap |
| layout-direction | Layout direction for widgets that support it | direction |
| align_horizontal | how to align children horizontally | align |
| align_vertical | how to align children vertically | align |
### Value syntax
| Type | Syntax | Notes |
|---|---|---|
| color | `#rgb`<br>`#rgba`<br>`#rrggbb`<br>`#rrggbbaa` | Examples:<br>`#fff`<br>`#ff00ff` |
| url | `"filename"` | An url in between quotes<br>`"image.png"`<br>`"font.ttf"` |
| number | floating point literal | A number, such as `2.0` or `42` |
| background | `<url>`<br>`<color>`<br>`image(<url>, <color>)`<br>`patch(<url>, <color>)`<br>`none` | If a url ends with `.9.png` it will be resolved  9 patch.<br>If your 9 slice doesn't end with `.9.png`, use `patch`. |
| rectangle | `<num>`<br>`<num> <num>`<br>`<num> <num> <num>`<br>`<num> <num> <num> <num>` | `all sides`<br>`top/bottom`, `right/left`<br>`top`, ht/left`, `bottom`<br>`top`, `right`, `bottom`, `left` |
| textwrap | `no-wrap`<br>`wrap`<br>`word-wrap` | |
| size | `<number>`<br>`fill(<number>)`<br>`exact(<number>)`<br>`shrink` | Just a number resolves to `exact` |
| direction | `top-to-bottom`<br>`left-to-right`<br>`right-to-left`<br>`bottom-to-top` | |
| align | `begin`<br>`center`<br>`end` | |