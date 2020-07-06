use std::ops::Deref;
use std::ops::DerefMut;

pub struct QTree<T> {
    size: usize,
    elements: Elems<T>,
    parent_index: usize,
}

#[derive(Debug, Clone)]
pub struct Area {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub size: usize,
}

enum Elems<T> {
    Node(Box<[QTree<T>; 4]>),
    Leaf(T, Area),
    Empty,
}

enum Action {
    Insert,
    Split,
    Recurse,
    Halt,
}

fn ceil_pow2(x: usize) -> usize {
    let mut r = 1;
    while r < x {
        r *= 2;
    }
    r
}

impl<T> QTree<T> {
    pub fn new(size: usize) -> Self {
        QTree {
            size: size,
            elements: Elems::Empty,
            parent_index: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn insert(&mut self, val: T, size: usize) -> Result<Area, ()> {
        let size = ceil_pow2(size);

        let area = Area {
            left: 0,
            top: 0,
            right: self.size,
            bottom: self.size,
            size: self.size,
        };

        assert!(size <= self.size);

        self.inner_insert(size, area).map(|result| {
            let (node, area) = result;

            // transform empty node into leaf
            node.elements = Elems::Leaf(val, area.clone());

            // return area that was inserted to
            area
        })
    }

    pub fn iter(&self) -> Iter<T> {
        let mut parents = Vec::new();
        let current = traverse(self, &mut parents, 0);
        Iter {
            current: current,
            parents: parents,
        }
    }

    fn inner_insert(&mut self, size: usize, area: Area) -> Result<(&mut QTree<T>, Area), ()> {
        let action = match &self.elements {
            &Elems::Empty => {
                if self.size > size {
                    Action::Split
                } else {
                    Action::Insert
                }
            }
            &Elems::Node(..) if self.size > size => Action::Recurse,
            _ => Action::Halt,
        };

        match action {
            Action::Split => {
                self.elements = Elems::Node(Box::new([
                    QTree {
                        size: self.size / 2,
                        elements: Elems::Empty,
                        parent_index: 0,
                    },
                    QTree {
                        size: self.size / 2,
                        elements: Elems::Empty,
                        parent_index: 1,
                    },
                    QTree {
                        size: self.size / 2,
                        elements: Elems::Empty,
                        parent_index: 2,
                    },
                    QTree {
                        size: self.size / 2,
                        elements: Elems::Empty,
                        parent_index: 3,
                    },
                ]));

                self.inner_insert(size, area)
            }
            Action::Insert => Ok((self, area)),
            Action::Recurse => match &mut self.elements {
                &mut Elems::Node(ref mut a) => {
                    let mut result = Err(());
                    for (i, x) in a.deref_mut().iter_mut().enumerate() {
                        result = x.inner_insert(
                            size,
                            Area {
                                left: if i % 2 == 0 {
                                    area.left
                                } else {
                                    (area.left + area.right) / 2
                                },
                                top: if i / 2 == 0 {
                                    area.top
                                } else {
                                    (area.top + area.bottom) / 2
                                },
                                right: if i % 2 == 0 {
                                    (area.left + area.right) / 2
                                } else {
                                    area.right
                                },
                                bottom: if i / 2 == 0 {
                                    (area.top + area.bottom) / 2
                                } else {
                                    area.bottom
                                },
                                size: area.size,
                            },
                        );

                        if result.is_ok() {
                            break;
                        }
                    }
                    result
                }
                _ => unreachable!(),
            },
            Action::Halt => Err(()),
        }
    }
}

fn traverse<'a, T>(node: &'a QTree<T>, parents: &mut Vec<&'a QTree<T>>, index: usize) -> Option<&'a QTree<T>> {
    let elems = &node.elements;

    match elems {
        &Elems::Node(ref a, ..) => {
            let slice = a.deref();
            match index {
                3 => {
                    let parent = parents.pop();
                    parent.map_or(None, |parent| traverse(parent, parents, node.parent_index + 1))
                }
                _ => {
                    parents.push(node);
                    traverse(&slice[index], parents, 0)
                }
            }
        }
        &Elems::Leaf(..) => Some(node),
        &Elems::Empty => {
            let parent = parents.pop();
            parent.map_or(None, |parent| traverse(parent, parents, node.parent_index + 1))
        }
    }
}

pub struct Iter<'a, T: 'a> {
    current: Option<&'a QTree<T>>,
    parents: Vec<&'a QTree<T>>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (&'a T, Area);

    fn next(&mut self) -> Option<(&'a T, Area)> {
        self.current.map(|tree| {
            let parent = self.parents.pop();
            self.current = parent.map_or(None, |parent| {
                traverse(parent, &mut self.parents, tree.parent_index + 1)
            });

            match &tree.elements {
                &Elems::Leaf(ref val, ref area) => (val, area.clone()),
                _ => unreachable!(),
            }
        })
    }
}
