use std::any::Any;

/// An [`Widget`](../widget/trait.Widget.html) state tracker.
pub struct ManagedState {
    state: Vec<Tracked>,
}

struct Tracked {
    id: u64,
    state: Box<dyn Any + Send + Sync>,
}

/// Temporary object used to find state objects for given ids.
pub struct ManagedStateTracker<'a> {
    tracker: &'a mut ManagedState,
    index: usize,
}

impl ManagedState {
    /// Retrieve a `ManagedStateTracker` that can be used to build a ui.
    /// Normally you will call this function at the start of your
    /// [`view`](../trait.Model.html#tymethod.view) implementation.
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
        let state = self
            .state
            .downcast_mut::<T>()
            .expect("widgets with the same id must always be of the same type");

        (state as *mut T).as_mut().unwrap()
    }
}

impl<'a> ManagedStateTracker<'a> {
    /// Get a state object for the given id. If such an object doesn't exist yet, it is constructed using it's `Default`
    /// implementation.
    pub fn get<'i, T>(&mut self, id: u64) -> &'i mut T
    where
        T: Default + Any + Send + Sync,
    {
        self.get_or_default_with(id, T::default)
    }

    /// Get a state object for the given id. If such an object doesn't exist yet, the supplied default value is used.
    pub fn get_or_default<'i, T>(&mut self, id: u64, default: T) -> &'i mut T
    where
        T: Any + Send + Sync,
    {
        self.get_or_default_with(id, move || default)
    }

    /// Get a state object for the given id. If such an object doesn't exist yet, it is constructed using the closure.
    pub fn get_or_default_with<'i, T, F>(&mut self, id: u64, default: F) -> &'i mut T
    where
        T: Any + Send + Sync,
        F: FnOnce() -> T,
    {
        let search_start = self.index;

        while self.index < self.tracker.state.len() {
            if self.tracker.state[self.index].id == id {
                self.tracker.state.drain(search_start..self.index).count();
                unsafe {
                    let i = search_start;
                    self.index = search_start + 1;
                    return self.tracker.state[i].unchecked_mut_ref();
                }
            } else {
                self.index += 1;
            }
        }

        self.tracker.state.insert(
            search_start,
            Tracked {
                id,
                state: Box::new(default()) as Box<dyn Any + Send + Sync>,
            },
        );
        self.index = search_start + 1;
        unsafe { self.tracker.state[search_start].unchecked_mut_ref() }
    }
}

impl<'a> Drop for ManagedStateTracker<'a> {
    fn drop(&mut self) {
        while self.index < self.tracker.state.len() {
            self.tracker.state.pop();
        }
    }
}
