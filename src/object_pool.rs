use std::vec::Vec;

/// One item in a datapool with a used flag
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Clone)]
struct ReusableItem<T> {
    reserved: bool,
    item: T,
}

#[cfg(feature = "serialization")]
use std::fs::File;
#[cfg(feature = "serialization")]
use std::io::Read;
#[cfg(feature = "serialization")]
use std::io::Write;

impl<
        #[cfg(feature = "serialization")] T: Default + serde::Serialize + serde::de::DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default,
    > ReusableItem<T>
{
    pub fn create(item: T) -> Self {
        Self {
            reserved: false,
            item,
        }
    }

    #[cfg(feature = "serialization")]
    pub fn write(&self, path: String) -> Result<(), std::io::Error> {
        let bytes = bendy::serde::to_bytes(self).ok().unwrap();
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    #[cfg(feature = "serialization")]
    pub fn read(path: String) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bendy::serde::from_bytes(&bytes).ok().unwrap())
    }
}

/// The key which identifies an element inside the ObjectPool
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ItemKey(usize);

impl Default for ItemKey {
    fn default() -> Self {
        ItemKey(usize::MAX)
    }
}

impl ItemKey {
    pub fn none_value() -> Self {
        ItemKey(usize::MAX)
    }
    pub fn is_some(&self) -> bool {
        self.0 < usize::MAX
    }
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

/// Stores re-usable objects to eliminate data allocation overhead when inserting and removing Nodes
/// It keeps track of different buffers for different levels in the graph, allocating more space initially to lower levels
#[derive(Default, Clone)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct ObjectPool<T: Default> {
    buffer: Vec<ReusableItem<T>>, // Pool of objects to be reused
    first_available: usize,       // the index of the first available item
}

impl<
        #[cfg(feature = "serialization")] T: Default + serde::Serialize + serde::de::DeserializeOwned,
        #[cfg(not(feature = "serialization"))] T: Default,
    > ObjectPool<T>
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
        ItemKey(key)
    }

    pub(crate) fn pop(&mut self, key: ItemKey) -> Option<T> {
        if key.0 < self.buffer.len() && self.buffer[key.0].reserved {
            self.buffer[key.0].reserved = false;
            self.first_available = self.first_available.min(key.0);
            Some(std::mem::take(&mut self.buffer[key.0].item))
        } else {
            None
        }
    }

    pub(crate) fn free(&mut self, key: ItemKey) -> bool {
        if key.0 < self.buffer.len() && self.buffer[key.0].reserved {
            self.buffer[key.0].reserved = false;
            self.first_available = self.first_available.min(key.0);
            true
        } else {
            false
        }
    }

    pub(crate) fn get(&self, key: ItemKey) -> &T {
        assert!(key.0 < self.buffer.len() && self.buffer[key.0].reserved);
        &self.buffer[key.0].item
    }

    pub(crate) fn get_mut(&mut self, key: ItemKey) -> &mut T {
        assert!(key.0 < self.buffer.len() && self.buffer[key.0].reserved);
        &mut self.buffer[key.0].item
    }

    #[cfg(feature = "serialization")]
    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        let bytes = bendy::serde::to_bytes(&self).ok().unwrap();
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    #[cfg(feature = "serialization")]
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bendy::serde::from_bytes(&bytes).ok().unwrap())
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

        pool.free(key);
        assert!(pool.pop(key).is_none());
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn test_reusable_item_file_io() {
        use super::ReusableItem;
        let item = ReusableItem::create(5.0);
        item.write("test_junk_item".to_string()).ok().unwrap();
        let cache_item = ReusableItem::read("test_junk_item".to_string())
            .ok()
            .unwrap();
        assert!(item.item == cache_item.item);
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn test_pool_file_io() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key = pool.push(test_value);
        pool.save("test_junk_pool").ok();

        let copy_pool = ObjectPool::<f32>::load("test_junk_pool").ok().unwrap();
        assert!(*copy_pool.get(key) == test_value);
    }
}
