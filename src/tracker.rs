use std::any::Any;
use std::borrow::Borrow;

/// An [`Widget`](../widget/trait.Widget.html) state tracker.
#[derive(Default)]
pub struct ManagedState<Id: Eq + Clone> {
    state: Vec<Tracked<Id>>,
}

struct Tracked<Id: Eq + Clone> {
    id: Id,
    state: Box<dyn Any>,
}

/// Temporary object used to find state objects for given ids.
pub struct ManagedStateTracker<'a, Id: Eq + Clone> {
    tracker: &'a mut ManagedState<Id>,
    index: usize,
}

impl<Id: Eq + Clone> ManagedState<Id> {
    /// Retrieve a `ManagedStateTracker` that can be used to build a ui.
    /// Normally you will call this function at the start of your
    /// [`view`](../trait.Model.html#tymethod.view) implementation.
    pub fn tracker(&mut self) -> ManagedStateTracker<Id> {
        ManagedStateTracker {
            tracker: self,
            index: 0,
        }
    }
}

impl<Id: Eq + Clone> Tracked<Id> {
    unsafe fn unchecked_mut_ref<'a, T: Any>(&mut self) -> &'a mut T {
        let state = self
            .state
            .downcast_mut::<T>()
            .expect("widgets with the same id must always be of the same type");

        (state as *mut T).as_mut().unwrap()
    }
}

impl<'a, Id: Eq + Clone> ManagedStateTracker<'a, Id> {
    /// Get a state object for the given id. If such an object doesn't exist yet, it is constructed using it's `Default`
    /// implementation.
    pub fn get<'i, T, Q>(&mut self, id: &Q) -> &'i mut T
    where
        T: Default + Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        Id: Borrow<Q>,
    {
        self.get_or_default_with(id, || T::default())
    }

    /// Get a state object for the given id. If such an object doesn't exist yet, the supplied default value is used.
    pub fn get_or_default<'i, T, Q>(&mut self, id: &Q, default: T) -> &'i mut T
    where
        T: Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        Id: Borrow<Q>,
    {
        self.get_or_default_with(id, move || default)
    }

    /// Get a state object for the given id. If such an object doesn't exist yet, it is constructed using the closure.
    pub fn get_or_default_with<'i, T, Q, F>(&mut self, id: &Q, default: F) -> &'i mut T
    where
        T: Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        F: FnOnce() -> T,
        Id: Borrow<Q>,
    {
        let search_start = self.index;

        while self.index < self.tracker.state.len() {
            if self.tracker.state[self.index].id.borrow().eq(id) {
                self.tracker.state.drain(search_start .. self.index).count();
                unsafe {
                    let i = search_start;
                    self.index = search_start + 1;
                    return self.tracker.state[i].unchecked_mut_ref();
                }
            } else {
                self.index += 1;
            }
        }

        self.tracker.state.insert(search_start, Tracked {
            id: id.to_owned(),
            state: Box::new(default()) as Box<dyn Any>,
        });
        self.index = search_start + 1;
        unsafe { self.tracker.state[search_start].unchecked_mut_ref() }
    }
}

impl<'a, Id: Eq + Clone> Drop for ManagedStateTracker<'a, Id> {
    fn drop(&mut self) {
        while self.index < self.tracker.state.len() {
            self.tracker.state.pop();
        }
    }
}
