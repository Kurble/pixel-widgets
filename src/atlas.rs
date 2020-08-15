use std::sync::Weak;

pub enum Atlas<T> {
    Split(Area, Box<[Atlas<T>; 4]>),
    Vacant(Area),
    Occupied(Area, T),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
}

impl<T> Atlas<T> {
    pub fn new(size: usize) -> Self {
        Atlas::Vacant(Area {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        })
    }

    pub fn size(&self) -> usize {
        match self {
            Atlas::Split(area, _) | Atlas::Vacant(area) | Atlas::Occupied(area, _) => area.right - area.left,
        }
    }

    pub fn area(&self) -> Area {
        match self {
            Atlas::Split(area, _) | Atlas::Vacant(area) | Atlas::Occupied(area, _) => area.clone(),
        }
    }

    pub fn insert(&mut self, mut val: T, size: usize) -> Result<Area, T> {
        let size = size.next_power_of_two();
        assert!(size <= self.size());

        match self {
            &mut Atlas::Split(_, ref mut children) => {
                for child in children.iter_mut() {
                    val = match child.insert(val, size) {
                        Ok(area) => return Ok(area),
                        Err(val) => val,
                    };
                }
                Err(val)
            }

            &mut Atlas::Occupied(_, _) => Err(val),

            vacant => {
                let area = vacant.area();
                if size < area.right - area.left {
                    *vacant = Atlas::Split(
                        area.clone(),
                        Box::new([
                            Atlas::Vacant(Area {
                                left: area.left,
                                top: area.top,
                                right: (area.left + area.right) / 2,
                                bottom: (area.top + area.bottom) / 2,
                            }),
                            Atlas::Vacant(Area {
                                left: (area.left + area.right) / 2,
                                top: area.top,
                                right: area.right,
                                bottom: (area.top + area.bottom) / 2,
                            }),
                            Atlas::Vacant(Area {
                                left: area.left,
                                top: (area.top + area.bottom) / 2,
                                right: (area.left + area.right) / 2,
                                bottom: area.bottom,
                            }),
                            Atlas::Vacant(Area {
                                left: (area.left + area.right) / 2,
                                top: (area.top + area.bottom) / 2,
                                right: area.right,
                                bottom: area.bottom,
                            }),
                        ]),
                    );

                    vacant.insert(val, size)
                } else {
                    *vacant = Atlas::Occupied(area.clone(), val);
                    Ok(area.clone())
                }
            }
        }
    }
}

impl<T> Atlas<Weak<T>> {
    pub fn remove_expired(&mut self) -> bool {
        let (area, empty) = match self {
            &mut Atlas::Split(ref area, ref mut children) => (
                area.clone(),
                children
                    .iter_mut()
                    .fold(true, |empty, child| child.remove_expired() && empty),
            ),
            &mut Atlas::Vacant(ref area) => (area.clone(), true),
            &mut Atlas::Occupied(ref area, ref content) => {
                if content.strong_count() == 0 {
                    (area.clone(), true)
                } else {
                    (area.clone(), false)
                }
            }
        };

        if empty {
            *self = Atlas::Vacant(area);
            true
        } else {
            false
        }
    }
}
