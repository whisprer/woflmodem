// src/audio/ringbuffer.rs
use std::sync::atomic::{AtomicUsize, Ordering};

/// Lock-free single-producer single-consumer ring buffer
pub struct RingBuffer<T: Copy + Default> {
    buffer: Vec<T>,
    capacity: usize,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![T::default(); capacity],
            capacity,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
        }
    }
    
    /// Write samples to buffer (SPSC safe)
    pub fn write(&self, data: &[T]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        let available = if write_pos >= read_pos {
            self.capacity - (write_pos - read_pos) - 1
        } else {
            read_pos - write_pos - 1
        };
        
        let to_write = data.len().min(available);
        
        unsafe {
            let buffer_ptr = self.buffer.as_ptr() as *mut T;
            for i in 0..to_write {
                let pos = (write_pos + i) % self.capacity;
                *buffer_ptr.add(pos) = data[i];
            }
        }
        
        self.write_pos.store((write_pos + to_write) % self.capacity, Ordering::Release);
        to_write
    }
    
    /// Read samples from buffer (SPSC safe)
    pub fn read(&self, data: &mut [T]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        let available = if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - (read_pos - write_pos)
        };
        
        let to_read = data.len().min(available);
        
        unsafe {
            let buffer_ptr = self.buffer.as_ptr();
            for i in 0..to_read {
                let pos = (read_pos + i) % self.capacity;
                data[i] = *buffer_ptr.add(pos);
            }
        }
        
        self.read_pos.store((read_pos + to_read) % self.capacity, Ordering::Release);
        to_read
    }
    
    /// Get current fill level
    pub fn available(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - (read_pos - write_pos)
        }
    }
}

unsafe impl<T: Copy + Default> Send for RingBuffer<T> {}
unsafe impl<T: Copy + Default> Sync for RingBuffer<T> {}
