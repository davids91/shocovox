use std::vec::Vec;

/// One item in a datapool with a used flag
#[derive(Clone)]
struct ReusableItem<T> {
    reserved: bool,
    item: T,
}

/// The key which identifies an element inside the ObjectPool
pub(crate) type ItemKey = usize;

/// Stores re-usable objects to eliminate data allocation overhead when inserting and removing Nodes
/// It keeps track of different buffers for different levels in the graph, allocating more space initially to lower levels
#[derive(Default, Clone)]
pub(crate) struct ObjectPool<T> {
    buffer: Vec<ReusableItem<T>>, // Pool of objects to be reused
    first_available: usize,       // the index of the first available item
}

use core::num::NonZeroU32;
impl<T> ObjectPool<T>
where
    T: Default,
{
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        ObjectPool {
            buffer: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    fn is_next_available(&mut self) -> bool {
        self.first_available + 1 < self.buffer.len()
            && !self.buffer[self.first_available + 1].reserved
    }

    fn check_first_available(&mut self) -> bool {
        if self.first_available < self.buffer.len() && !self.buffer[self.first_available].reserved {
            true
        } else if self.is_next_available() {
            self.first_available += 1;
            true
        } else {
            self.first_available = self.buffer.len();
            false
        }
    }

    pub(crate) fn push(&mut self, item: T) -> ItemKey {
        let key = self.allocate();
        *self.get_mut(key) = item;
        key
    }

    pub(crate) fn allocate(&mut self) -> ItemKey {
        let key = if self.check_first_available() {
            self.first_available
        } else {
            // reserve place for additional items
            let x = self.buffer.len().max(10) as f32;

            // reserve less additional items the more the size of the buffer
            self.buffer
                .reserve(((100. * x.log10().powf(2.)) / x) as usize);

            // mark Node as reserved and return with the key
            self.buffer.push(ReusableItem {
                reserved: true,
                item: T::default(),
            });
            self.buffer.len() - 1
        };
        if self.is_next_available() {
            self.first_available += 1;
        }
        key
    }

    pub(crate) fn pop(&mut self, key: ItemKey) -> Option<T> {
        if key < self.buffer.len() && self.buffer[key].reserved {
            self.buffer[key].reserved = false;
            self.first_available = self.first_available.min(key);
            Some(std::mem::replace(
                &mut self.buffer[key].item,
                Default::default(),
            ))
        } else {
            None
        }
    }

    pub(crate) fn deallocate(&mut self, key: ItemKey) -> bool {
        if key < self.buffer.len() && self.buffer[key].reserved {
            self.buffer[key].reserved = false;
            self.first_available = self.first_available.min(key);
            true
        } else {
            false
        }
    }

    pub(crate) fn get(&self, key: ItemKey) -> &T {
        &self.buffer[key].item
    }

    pub(crate) fn get_mut(&mut self, key: ItemKey) -> &mut T {
        assert!(key < self.buffer.len() && self.buffer[key].reserved);
        &mut self.buffer[key].item
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectPool;

    #[test]
    fn test_push_pop_modify() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key = pool.push(test_value);
        assert!(*pool.get(key) == test_value);

        *pool.get_mut(key) = 10.;
        assert!(*pool.get(key) == 10.);

        assert!(pool.pop(key).unwrap() == 10.);
        assert!(pool.pop(key).is_none());
    }

    #[test]
    fn test_push_deallocate() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key = pool.push(test_value);
        assert!(*pool.get(key) == test_value);

        pool.deallocate(key);
        assert!(pool.pop(key).is_none());
    }
}
