# Changelog

### v0.5.8
- Fixed panic bug with stylesheets that have more than 64 rules

### v0.5.7
- Fixed panic bug with stylesheets that have more than 32 rules

### v0.5.6
- Bump winit to 0.24

### v0.5.4
- Fixed bug where some async tasks were resumed after being finished

### v0.5.3
- Fixed bug where ui wouldn't be redrawn even if needed

### v0.5.2
- Added input::State::get_value and updated the set_value return value.

### v0.5.1
- Fixed a bug with inserting textures in the atlas

### v0.5.0
- More `Loader` flexibility.
- `Style` is responsible for textures instead of `Ui`

### v0.4.3
- Fixed `ManagedState` not working anymore

### v0.4.2 (yanked)
- Made all widgets and `Ui` `Send` compatible

### v0.4.1
- Fixed compilation errors after dependencies that were allowed to updated were updated.

### v0.4.0
- `Model::update` now returns a `Vec<Command<Message>>`, which can be used to send async messages.
- `Ui::command` added, which can be used to send an async message externally
- Download example added
- `Ui::reload_stylesheet` added.
- Loader system has been refactored
- Margins added to stylesheet system. Margins automatically handled for all widgets.
- Added `widget::input::State::set_value`
- Removed scrollbars from stylesheet in favor of the new `Dummy` widget.
- Added `Progress` widget.
- Added support for flags to stylesheets.
- Added `Menu` widget.
- Added `on_right_click` callback to `Node`.
- Modified `Widget::state` to return a `SmallVec` of states, to support multiple states at once, 
like a `Toggle` than be `checked` and `hover` at the same time.
- Added `Drag` and `Drop` widget.
- The `Layers` widget now propagates events to all layers, except for `Event::Cursor`.

 ### v0.3.0
- Added `len()` to `Widget`.
- New style system
    - Changed pwss syntax to resemble css more.
    - Removed some backgrounds from [`Stylesheet`](src/stylesheet/mod.rs).
    The styling system is now responsible for specifying these using selectors like `:hover`.
    - Added `:nth-first-child(n)`, `:nth-last-child(n)`, `:nth-first-child-mod(n, d)`, 
    `:nth-last-child-mod(n, d)` selectors. All support numbers, `odd` and `even`.
    - Added `:first-child`, `:last-child` and `:only-child` selectors.
    - Added `:not(<selector>)` selector.
    - Added a `:<state>` selector that checks the result of the new method `Widget::state()`.
    Useful for states such as `hover`, `pressed` or `open`.
    - Added `+ <widget>`, `> <widget>` and `~ <widget>` selectors.
    - The any (`*`) selector can now be used in any place where `<widget>` is expected.  

 ### v0.2.0
- Added a sandbox module so you don't need to initialize a window yourself
- Fixed bugs in the [`ManagedStateTracker`](src/tracker.rs)
- Added padding behaviour to `Button`, `Column`, `Dropdown`, `Row`, `Scroll`, `Text` and `Window` widgets
- Added a system for widgets to take exclusive focus
- Added [`Dropdown`](src/widget/dropdown.rs) widget
- Fixed build errors when turning off the features

 ### v0.1.1
- Fixed docs.rs. build