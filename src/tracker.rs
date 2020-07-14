use std::any::Any;
use std::borrow::Borrow;

#[derive(Default)]
pub struct ManagedState<Id: Eq + Clone> {
    state: Vec<Tracked<Id>>,
}

struct Tracked<Id: Eq + Clone> {
    id: Id,
    state: Box<dyn Any>,
}

pub struct ManagedStateTracker<'a, Id: Eq + Clone> {
    tracker: &'a mut ManagedState<Id>,
    index: usize,
}

impl<Id: Eq + Clone> ManagedState<Id> {
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
            .expect("elements with the same id must always be of the same type");

        (state as *mut T).as_mut().unwrap()
    }
}

impl<'a, Id: Eq + Clone> ManagedStateTracker<'a, Id> {
    pub fn get<'i, T, Q>(&mut self, id: &Q) -> &'i mut T
    where
        T: Default + Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        Id: Borrow<Q>,
    {
        self.get_or_default_with(id, || T::default())
    }

    pub fn get_or_default<'i, T, Q>(&mut self, id: &Q, default: T) -> &'i mut T
    where
        T: Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        Id: Borrow<Q>,
    {
        self.get_or_default_with(id, move || default)
    }

    pub fn get_or_default_with<'i, T, Q, F>(&mut self, id: &Q, default: F) -> &'i mut T
    where
        T: Any,
        Q: ?Sized + Eq + ToOwned<Owned = Id>,
        F: FnOnce() -> T,
        Id: Borrow<Q>,
    {
        while self.index < self.tracker.state.len() {
            if self.tracker.state[self.index].id.borrow().eq(id) {
                unsafe {
                    let i = self.index;
                    self.index += 1;
                    return self.tracker.state[i].unchecked_mut_ref();
                }
            } else {
                self.tracker.state.remove(self.index);
            }
        }
        self.index += 1;
        self.tracker.state.push(Tracked {
            id: id.to_owned(),
            state: Box::new(default()) as Box<dyn Any>,
        });
        unsafe { self.tracker.state.last_mut().unwrap().unchecked_mut_ref() }
    }
}

impl<'a, Id: Eq + Clone> Drop for ManagedStateTracker<'a, Id> {
    fn drop(&mut self) {
        while self.index < self.tracker.state.len() {
            self.tracker.state.pop();
        }
    }
}
