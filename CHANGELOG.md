# Changelog

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