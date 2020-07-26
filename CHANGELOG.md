# Changelog

 ### v0.3.0
- New style system
    - Removed some backgrounds from [`Stylesheet`](src/stylesheet/mod.rs).
    The styling system is now responsible for specifying these using selectors like `:hover`.
    - Added `:odd`, `:even`, `:nth(n)`, `:first`, `:last` selectors.
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