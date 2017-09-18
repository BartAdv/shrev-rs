//! Ring buffer implementation, that does immutable reads.

use std::ops::{Index, IndexMut};
use std::any::TypeId;
use std::fmt::Debug;

/// Ringbuffer errors
#[derive(Debug)]
pub enum RBError<T: Debug> {
    /// If a writer tries to write more data than the max size of the ringbuffer, in a single call
    TooLargeWrite,
    /// If a reader is more than the entire ringbuffer behind in reading, this will be returned.
    /// Contains the data that could be salvaged
    LostData(Vec<T>, usize),
    /// If attempting to use a reader for a different data type than the storage contains.
    InvalidReader,
}

/// The reader id is used by readers to tell the storage where the last read ended.
#[derive(Hash, PartialEq, Copy, Clone, Debug)]
pub struct ReaderId {
    t: TypeId,
    id: u32,
    read_index: usize,
    written: usize,
}

impl ReaderId {
    /// Create a new reader id
    pub fn new(t: TypeId, id: u32, reader_index: usize, written: usize) -> ReaderId {
        ReaderId {
            t: t,
            id: id,
            read_index: reader_index,
            written: written,
        }
    }
}

/// Ring buffer, holding data of type `T`
pub struct RingBufferStorage<T: Debug> {
    data: Vec<T>,
    write_index: usize,
    max_size: usize,
    written: usize,
    next_reader_id: u32,
    reset_written: usize,
}

impl<T: Clone + 'static + Debug> RingBufferStorage<T> {

    /// Create a new ring buffer with the given max size.
    pub fn new(size: usize) -> Self {
        RingBufferStorage {
            data: Vec::with_capacity(size),
            write_index: 0,
            max_size: size,
            written: 0,
            next_reader_id: 1,
            reset_written: size * 1000,
        }
    }

    /// Write a set of data into the ringbuffer.
    pub fn write(&mut self, data: &mut Vec<T>) -> Result<(), RBError<T>> {
        if data.len() == 0 {
            return Ok(());
        }
        if data.len() > self.max_size {
            return Err(RBError::TooLargeWrite);
        }
        for d in data.drain(0..) {
            self.write_single(d);
        }
        Ok(())
    }

    /// Write a single data point into the ringbuffer.
    pub fn write_single(&mut self, data: T) {
        let mut write_index = self.write_index;
        if write_index == self.data.len() {
            self.data.push(data);
        } else {
            self.data[write_index] = data;
        }
        write_index += 1;
        if write_index >= self.max_size {
            write_index = 0;
        }
        self.write_index = write_index;
        self.written += 1;
        if self.written > self.reset_written {
            self.written = 0;
        }
    }

    /// Create a new reader id for this ringbuffer.
    pub fn new_reader_id(&mut self) -> ReaderId {
        let reader_id = ReaderId::new(
            TypeId::of::<T>(),
            self.next_reader_id,
            self.write_index,
            self.written,
        );
        self.next_reader_id += 1;
        reader_id
    }

    /// Read data from the ringbuffer, starting where the last read ended, and up to where the last
    /// data was written.
    pub fn read(&self, reader_id: &mut ReaderId) -> Result<Vec<T>, RBError<T>> {
        if reader_id.t != TypeId::of::<T>() {
            return Err(RBError::InvalidReader);
        }
        let num_written = if self.written < reader_id.written {
            self.written + (self.reset_written - reader_id.written)
        } else {
            self.written - reader_id.written
        };
        if num_written > self.max_size {
            let mut d = self.data
                .get(self.write_index..self.max_size)
                .unwrap()
                .to_vec();
            d.extend(self.data.get(0..self.write_index).unwrap().to_vec());
            reader_id.read_index = self.write_index;
            reader_id.written = self.written;
            return Err(RBError::LostData(d, num_written - self.max_size));
        }
        let read_data = if self.write_index >= reader_id.read_index {
            self.data
                .get(reader_id.read_index..self.write_index)
                .unwrap()
                .to_vec()
        } else {
            let mut d = self.data
                .get(reader_id.read_index..self.max_size)
                .unwrap()
                .to_vec();
            d.extend(self.data.get(0..self.write_index).unwrap().to_vec());
            d
        };
        reader_id.read_index = self.write_index;
        reader_id.written = self.written;
        Ok(read_data)
    }
}

impl<T: Debug> Index<usize> for RingBufferStorage<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T: Debug> IndexMut<usize> for RingBufferStorage<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}