use std::any::Any;

/// An [`Widget`](../widget/trait.Widget.html) state tracker.
pub(crate) struct ManagedState {
    state: Vec<Tracked>,
}

enum Tracked {
    Begin { id: u64, state: Box<dyn Any + Send + Sync> },
    End,
}

#[doc(hidden)]
pub struct ManagedStateTracker<'a> {
    tracker: &'a mut ManagedState,
    index: usize,
}

impl ManagedState {
    /// Retrieve a `ManagedStateTracker` that can be used to build a ui.
    /// Normally you will call this function at the start of your
    /// [`view`](../trait.Component.html#tymethod.view) implementation.
    pub fn tracker(&mut self) -> ManagedStateTracker {
        ManagedStateTracker {
            tracker: self,
            index: 0,
        }
    }
}

impl Default for ManagedState {
    fn default() -> Self {
        Self { state: Vec::new() }
    }
}

impl Tracked {
    unsafe fn unchecked_mut_ref<'a, T: Any + Send + Sync>(&mut self) -> &'a mut T {
        match self {
            Tracked::Begin { state, .. } => {
                let state = state
                    .downcast_mut::<T>()
                    .expect("widgets with the same id must always be of the same type");

                (state as *mut T).as_mut().unwrap()
            }
            _ => unreachable!(),
        }
    }
}

impl<'a> ManagedStateTracker<'a> {
    /// Get a state object for the given id. If such an object doesn't exist yet, it is constructed using the closure.
    /// The span of the widget that requests this state object should be closed using [`end`](#method.end).
    pub(crate) fn begin<'i, T, F>(&mut self, id: u64, default: F) -> &'i mut T
    where
        T: Any + Send + Sync,
        F: FnOnce() -> T,
    {
        let search_start = self.index;
        let mut level = 0;

        while self.index < self.tracker.state.len() {
            match &self.tracker.state[self.index] {
                Tracked::End if level > 0 => level -= 1,
                Tracked::End => {
                    // not found, revert to start of local scope
                    self.index = search_start;
                    break;
                }
                &Tracked::Begin { id: tid, state: _ } if level == 0 && tid == id => {
                    self.tracker.state.splice(search_start..self.index, None);
                    unsafe {
                        let i = search_start;
                        self.index = search_start + 1;
                        return self.tracker.state[i].unchecked_mut_ref();
                    }
                }
                &Tracked::Begin { .. } => level += 1,
            }
            self.index += 1;
        }

        let i = self.index;
        let state = Box::new(default()) as Box<dyn Any + Send + Sync>;
        self.tracker.state.insert(i, Tracked::Begin { id, state });
        self.tracker.state.insert(i + 1, Tracked::End);
        self.index += 1;
        unsafe { self.tracker.state[i].unchecked_mut_ref() }
    }

    /// Ends the span of a widget.
    /// Should be called after all of it's children have been handled.
    pub(crate) fn end(&mut self) {
        let search_start = self.index;
        let mut level = 0;

        while self.index < self.tracker.state.len() {
            match &self.tracker.state[self.index] {
                Tracked::Begin { .. } => {
                    self.index += 1;
                    level += 1;
                }
                Tracked::End if level > 0 => {
                    self.index += 1;
                    level -= 1;
                }
                Tracked::End => {
                    // found it! remove any widget states that were not matched.
                    self.tracker.state.splice(search_start..self.index, None);
                    self.index = search_start + 1;
                    return;
                }
            }
        }

        unreachable!("did not find `End` at the end.");
    }
}

impl<'a> Drop for ManagedStateTracker<'a> {
    fn drop(&mut self) {
        while self.index < self.tracker.state.len() {
            self.tracker.state.pop();
        }
    }
}
