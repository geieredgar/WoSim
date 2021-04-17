use std::{
    mem::{size_of, swap},
    sync::Arc,
};

use ash::vk::{self, BufferCreateInfo, BufferUsageFlags};
use vk_mem::{
    Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, MemoryUsage,
};

use super::Device;

pub struct Buffer {
    pub(super) handle: vk::Buffer,
    allocation: Allocation,
    device: Arc<Device>,
}

impl Buffer {
    pub fn new(
        device: Arc<Device>,
        create_info: &BufferCreateInfo,
        allocation_create_info: &AllocationCreateInfo,
    ) -> vk_mem::Result<(Self, AllocationInfo)> {
        let (handle, allocation, allocation_info) = device
            .allocator
            .create_buffer(create_info, allocation_create_info)?;
        Ok((
            Self {
                handle,
                allocation,
                device,
            },
            allocation_info,
        ))
    }

    pub fn flush(&self, offset: usize, size: usize) -> vk_mem::Result<()> {
        self.device
            .allocator
            .flush_allocation(&self.allocation, offset, size)
    }

    pub fn invalidate(&self, offset: usize, size: usize) -> vk_mem::Result<()> {
        self.device
            .allocator
            .invalidate_allocation(&self.allocation, offset, size)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device
            .allocator
            .destroy_buffer(self.handle, &self.allocation)
            .unwrap()
    }
}

pub struct GpuVariable<T: Copy> {
    buffer: Buffer,
    ptr: *mut T,
}

impl<T: Copy> GpuVariable<T> {
    pub fn new(
        device: Arc<Device>,
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        value: T,
    ) -> vk_mem::Result<Self> {
        let create_info = BufferCreateInfo::builder()
            .size(size_of::<T>() as u64)
            .usage(buffer_usage);
        let allocation_create_info = AllocationCreateInfo {
            usage: memory_usage,
            flags: AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (buffer, info) = Buffer::new(device, &create_info, &&allocation_create_info)?;
        let ptr = info.get_mapped_data() as *mut T;
        unsafe { ptr.write(value) };
        Ok(Self { buffer, ptr })
    }

    pub fn flush(&self) -> vk_mem::Result<()> {
        self.buffer.flush(0, size_of::<T>())
    }

    pub fn invalidate(&self) -> vk_mem::Result<()> {
        self.buffer.invalidate(0, size_of::<T>())
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn value(&self) -> &T {
        unsafe { self.ptr.as_ref() }.unwrap()
    }

    pub fn value_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }.unwrap()
    }
}

pub struct GpuVec<T: Copy> {
    buffer: Buffer,
    len: usize,
    capacity: usize,
    buffer_usage: BufferUsageFlags,
    memory_usage: MemoryUsage,
    ptr: *mut T,
}

impl<T: Copy> GpuVec<T> {
    pub fn new(
        device: Arc<Device>,
        capacity: usize,
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
    ) -> vk_mem::Result<Self> {
        assert_ne!(capacity, 0);
        let create_info = BufferCreateInfo::builder()
            .size((size_of::<T>() * capacity) as u64)
            .usage(buffer_usage);
        let allocation_create_info = AllocationCreateInfo {
            usage: memory_usage,
            flags: AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (buffer, info) = Buffer::new(device, &create_info, &allocation_create_info)?;
        let ptr = info.get_mapped_data() as *mut T;
        Ok(Self {
            buffer,
            len: 0,
            capacity,
            ptr,
            buffer_usage,
            memory_usage,
        })
    }

    pub fn push(&mut self, value: T) {
        assert!(self.len < self.capacity);
        unsafe { self.ptr.add(self.len).write(value) };
    }

    pub fn reserve(&mut self, capacity: usize) -> vk_mem::Result<()> {
        if self.capacity >= capacity {
            return Ok(());
        }
        let create_info = BufferCreateInfo::builder()
            .size((size_of::<T>() * capacity) as u64)
            .usage(self.buffer_usage);
        let allocation_create_info = AllocationCreateInfo {
            usage: self.memory_usage,
            flags: AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (mut buffer, info) = Buffer::new(
            self.buffer.device.clone(),
            &create_info,
            &allocation_create_info,
        )?;
        let mut ptr = info.get_mapped_data() as *mut T;
        unsafe { ptr.copy_from_nonoverlapping(self.ptr, self.len) };
        swap(&mut self.buffer, &mut buffer);
        swap(&mut self.ptr, &mut ptr);
        self.capacity = capacity;
        Ok(())
    }

    pub fn append(&mut self, values: &[T]) {
        let new_len = self.len + values.len();
        assert!(new_len <= self.capacity);
        unsafe {
            self.ptr
                .add(self.len)
                .copy_from_nonoverlapping(values.as_ptr(), values.len())
        };
        self.len = new_len;
    }

    pub fn flush(&self) -> vk_mem::Result<()> {
        self.buffer.flush(0, self.len * size_of::<T>())
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}
