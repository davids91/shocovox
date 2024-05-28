use std::vec::Vec;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// One item in a datapool with a used flag
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[derive(Clone)]
struct ReusableItem<T: Clone> {
    reserved: bool,
    item: T,
}

pub fn key_none_value() -> u32 {
    u32::MAX
}

use bendy::encoding::{Error as BencodeError, SingleItemEncoder, ToBencode};
impl<T> ToBencode for ReusableItem<T>
where
    T: Clone + ToBencode,
{
    const MAX_DEPTH: usize = 6;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        encoder.emit_list(|e| {
            e.emit_int(self.reserved as u8)?;
            e.emit(self.item.clone())
        })
    }
}

use bendy::decoding::{FromBencode, Object};
impl<T> FromBencode for ReusableItem<T>
where
    T: Clone + FromBencode,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let reserved = match list.next_object()?.unwrap() {
                    Object::Integer("0") => Ok(false),
                    Object::Integer("1") => Ok(true),
                    Object::Integer(i) => Err(bendy::decoding::Error::unexpected_token(
                        "boolean field reserved",
                        format!("the number: {}", i),
                    )),
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "boolean field reserved",
                        "Something else",
                    )),
                }?;
                let item = T::decode_bencode_object(list.next_object()?.unwrap())?;
                Ok(Self { item, reserved })
            }
            _ => Err(bendy::decoding::Error::unexpected_token(
                "List of ReusableItem<T> fields",
                "Something else",
            )),
        }
    }
}

///####################################################################################
/// ObjectPool
///####################################################################################

/// Stores re-usable objects to eliminate data allocation overhead when inserting and removing Nodes
/// It keeps track of different buffers for different levels in the graph, allocating more space initially to lower levels
#[derive(Default, Clone)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) struct ObjectPool<T: Clone> {
    buffer: Vec<ReusableItem<T>>, // Pool of objects to be reused
    first_available: usize,       // the index of the first available item
}

impl<
        #[cfg(not(feature = "serialization"))] T: Default + Clone + ToBencode,
        #[cfg(feature = "serialization")] T: Default + Clone + ToBencode,
    > ToBencode for ObjectPool<T>
{
    const MAX_DEPTH: usize = 8;
    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), BencodeError> {
        encoder.emit_list(|e| {
            e.emit_int(self.first_available)?;
            e.emit(&self.buffer)
        })
    }
}

impl<T> FromBencode for ObjectPool<T>
where
    T: Default + Clone + FromBencode,
{
    fn decode_bencode_object(data: Object) -> Result<Self, bendy::decoding::Error> {
        match data {
            Object::List(mut list) => {
                let first_available = match list.next_object()?.unwrap() {
                    Object::Integer(i) => Ok(i.parse::<usize>().ok().unwrap()),
                    _ => Err(bendy::decoding::Error::unexpected_token(
                        "int field first_available",
                        "Something else",
                    )),
                }?;
                let buffer = Vec::decode_bencode_object(list.next_object()?.unwrap())?;
                Ok(Self {
                    first_available,
                    buffer,
                })
            }
            _ => Err(bendy::decoding::Error::unexpected_token(
                "List of ObjectPool<T> fields",
                "Something else",
            )),
        }
    }
}

#[allow(dead_code)] // Object implemented for universal usage
impl<T> ObjectPool<T>
where
    T: Default + Clone,
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

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn push(&mut self, item: T) -> usize {
        let key = self.allocate();
        *self.get_mut(key) = item;
        key
    }

    pub(crate) fn allocate(&mut self) -> usize {
        let key = if self.check_first_available() {
            self.buffer[self.first_available].reserved = true;
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

    pub(crate) fn pop(&mut self, key: usize) -> Option<T> {
        if self.key_is_valid(key) {
            self.buffer[key].reserved = false;
            self.first_available = self.first_available.min(key);
            Some(std::mem::take(&mut self.buffer[key].item))
        } else {
            None
        }
    }

    pub(crate) fn free(&mut self, key: usize) -> bool {
        if self.key_is_valid(key) {
            self.buffer[key].reserved = false;
            self.first_available = self.first_available.min(key);
            true
        } else {
            false
        }
    }

    pub(crate) fn get(&self, key: usize) -> &T {
        debug_assert!(self.key_is_valid(key));
        &self.buffer[key].item
    }

    pub(crate) fn get_mut(&mut self, key: usize) -> &mut T {
        debug_assert!(self.key_is_valid(key));
        &mut self.buffer[key].item
    }

    pub(crate) fn key_is_valid(&self, key: usize) -> bool {
        key < self.buffer.len() && self.buffer[key].reserved
    }
}

#[cfg(test)]
mod object_pool_tests {
    use super::ObjectPool;

    #[test]
    fn test_push_pop_modify() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key = pool.push(test_value);
        debug_assert!(*pool.get(key) == test_value);

        *pool.get_mut(key) = 10.;
        debug_assert!(*pool.get(key) == 10.);

        debug_assert!(pool.pop(key).unwrap() == 10.);
        debug_assert!(pool.pop(key).is_none());
    }

    #[test]
    fn test_push_deallocate() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key = pool.push(test_value);
        debug_assert!(*pool.get(key) == test_value);

        pool.free(key);
        debug_assert!(pool.pop(key).is_none());
    }

    #[test]
    fn test_edge_case_reused_item() {
        let mut pool = ObjectPool::<f32>::with_capacity(3);
        let test_value = 5.;
        let key_1 = pool.push(test_value);
        pool.push(test_value * 2.);
        pool.pop(key_1);
        debug_assert!(pool.first_available == 0); // the first item should be available

        pool.push(test_value * 3.);
        debug_assert!(*pool.get(key_1) == test_value * 3.); // the original key is reused to hold the latest value
    }
}
