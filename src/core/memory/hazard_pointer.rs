use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

const MAX_HAZARD_POINTERS_PER_THREAD: usize = 8;
const RETIRE_THRESHOLD: usize = 32;
// Cache line size for alignment optimization
const CACHE_LINE_SIZE: usize = 64;

#[repr(align(64))] // CACHE_LINE_SIZE alignment for performance
struct CacheAligned<T>(T);

pub struct HazardPointerDomain {
    inner: Arc<HazardPointerDomainInner>,
}

impl std::fmt::Debug for HazardPointerDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HazardPointerDomain")
            .field("hazard_slots", &self.inner.hazard_pointers.len())
            .field(
                "active_threads",
                &self.inner.active_threads.load(Ordering::Relaxed),
            )
            .finish()
    }
}

struct HazardPointerDomainInner {
    hazard_pointers: Vec<HazardPointerSlot>,
    thread_data: Mutex<Vec<Arc<ThreadData>>>,
    global_retire_list: SegQueue<RetiredNode>,
    active_threads: AtomicUsize,
}

#[repr(C, align(64))] // Align to CACHE_LINE_SIZE for performance
struct HazardPointerSlot {
    pointer: CacheAligned<AtomicPtr<u8>>,
    active: AtomicBool,
    owner_thread_id: AtomicUsize,
}

impl HazardPointerSlot {
    // Validate that our alignment meets cache line requirements at compile time
    const ALIGNMENT_CHECK: () = {
        assert!(
            std::mem::align_of::<Self>() >= CACHE_LINE_SIZE,
            "HazardPointerSlot alignment must be at least cache line size"
        );
    };

    // Force the check to be evaluated
    #[allow(dead_code)]
    fn validate_alignment() -> bool {
        let _ = Self::ALIGNMENT_CHECK;
        std::mem::align_of::<Self>() >= CACHE_LINE_SIZE
    }
}

unsafe impl Send for HazardPointerDomainInner {}
unsafe impl Sync for HazardPointerDomainInner {}

struct ThreadData {
    thread_id: usize,
    local_retire_list: UnsafeCell<Vec<RetiredNode>>,
    hazard_indices: parking_lot::Mutex<Vec<usize>>,
}

struct RetiredNode {
    ptr: NonNull<u8>,
    deleter: Box<dyn FnOnce() + Send + 'static>,
}

unsafe impl Send for ThreadData {}
unsafe impl Sync for ThreadData {}

impl HazardPointerDomain {
    pub fn new(max_threads: usize) -> Self {
        let total_slots = max_threads * MAX_HAZARD_POINTERS_PER_THREAD;
        let mut hazard_pointers = Vec::with_capacity(total_slots);

        for _ in 0..total_slots {
            hazard_pointers.push(HazardPointerSlot {
                pointer: CacheAligned(AtomicPtr::new(std::ptr::null_mut())),
                active: AtomicBool::new(false),
                owner_thread_id: AtomicUsize::new(0),
            });
        }

        Self {
            inner: Arc::new(HazardPointerDomainInner {
                hazard_pointers,
                thread_data: Mutex::new(Vec::new()),
                global_retire_list: SegQueue::new(),
                active_threads: AtomicUsize::new(0),
            }),
        }
    }

    pub fn acquire(&self) -> HazardPointer<'_> {
        let thread_id = self.get_or_create_thread_id();
        let slot_index = self.find_free_slot(thread_id);

        // Track this hazard pointer index in the thread data
        if let Some(thread_data) = self.find_thread_data(thread_id) {
            thread_data.hazard_indices.lock().push(slot_index);
        }

        HazardPointer {
            domain: self,
            slot_index,
            thread_id,
        }
    }

    fn find_free_slot(&self, thread_id: usize) -> usize {
        let start = (thread_id * MAX_HAZARD_POINTERS_PER_THREAD) % self.inner.hazard_pointers.len();

        for i in 0..self.inner.hazard_pointers.len() {
            let idx = (start + i) % self.inner.hazard_pointers.len();
            let slot = &self.inner.hazard_pointers[idx];

            if !slot.active.load(Ordering::Acquire) {
                if slot
                    .active
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    slot.owner_thread_id.store(thread_id, Ordering::Release);
                    return idx;
                }
            }
        }

        panic!("No free hazard pointer slots available");
    }

    fn get_or_create_thread_id(&self) -> usize {
        thread_local! {
            static THREAD_ID: UnsafeCell<MaybeUninit<usize>> = UnsafeCell::new(MaybeUninit::uninit());
        }

        THREAD_ID.with(|id| unsafe {
            let id_ptr = id.get();
            // Check if thread ID is already initialized
            if (*id_ptr).as_ptr().read() == 0 {
                let new_id = self.inner.active_threads.fetch_add(1, Ordering::Relaxed) + 1;
                (*id_ptr).write(new_id);

                // Create thread data for this new thread
                let thread_data = Arc::new(ThreadData {
                    thread_id: new_id,
                    local_retire_list: UnsafeCell::new(Vec::new()),
                    hazard_indices: parking_lot::Mutex::new(Vec::with_capacity(
                        MAX_HAZARD_POINTERS_PER_THREAD,
                    )),
                });

                self.inner.thread_data.lock().push(thread_data);
                new_id
            } else {
                (*id_ptr).assume_init()
            }
        })
    }

    pub fn retire_ptr(&self, ptr: NonNull<u8>, size: usize, align: usize) {
        let thread_id = self.get_or_create_thread_id();
        let ptr_addr = ptr.as_ptr() as usize;
        let deleter: Box<dyn FnOnce() + Send + 'static> = Box::new(move || unsafe {
            if let Ok(layout) = std::alloc::Layout::from_size_align(size, align) {
                let ptr_to_dealloc = ptr_addr as *mut u8;
                if !ptr_to_dealloc.is_null() {
                    std::alloc::dealloc(ptr_to_dealloc, layout);
                }
            }
        });

        let retired = RetiredNode { ptr, deleter };

        // Try to find this thread's local retire list first
        if let Some(thread_data) = self.find_thread_data(thread_id) {
            unsafe {
                let local_list = &mut *thread_data.local_retire_list.get();
                local_list.push(retired);

                // If local list gets too big, move to global list
                if local_list.len() >= RETIRE_THRESHOLD / 2 {
                    for node in local_list.drain(..) {
                        self.inner.global_retire_list.push(node);
                    }
                    self.try_reclaim();
                }
            }
        } else {
            // Fallback to global list if thread data not found
            self.inner.global_retire_list.push(retired);
            if self.inner.global_retire_list.len() >= RETIRE_THRESHOLD {
                self.try_reclaim();
            }
        }
    }

    fn find_thread_data(&self, thread_id: usize) -> Option<Arc<ThreadData>> {
        let thread_data_list = self.inner.thread_data.lock();
        thread_data_list
            .iter()
            .find(|data| data.thread_id == thread_id)
            .map(Arc::clone)
    }

    fn try_reclaim(&self) {
        let mut hazard_set = HashSet::new();

        for slot in &self.inner.hazard_pointers {
            if slot.active.load(Ordering::Acquire) {
                let ptr = slot.pointer.0.load(Ordering::Acquire);
                if !ptr.is_null() {
                    hazard_set.insert(ptr as usize);
                }
            }
        }

        let mut deferred = Vec::new();

        while let Some(retired) = self.inner.global_retire_list.pop() {
            if hazard_set.contains(&(retired.ptr.as_ptr() as usize)) {
                deferred.push(retired);
            } else {
                (retired.deleter)();
            }
        }

        for node in deferred {
            self.inner.global_retire_list.push(node);
        }
    }
}

pub struct HazardPointer<'a> {
    domain: &'a HazardPointerDomain,
    slot_index: usize,
    thread_id: usize,
}

impl<'a> HazardPointer<'a> {
    pub fn protect<T>(&self, ptr: *const T) -> bool {
        let slot = &self.domain.inner.hazard_pointers[self.slot_index];
        slot.pointer.0.store(ptr as *mut u8, Ordering::Release);

        std::sync::atomic::fence(Ordering::SeqCst);

        true
    }

    /// Get the thread ID that owns this hazard pointer (used for debugging)
    pub fn thread_id(&self) -> usize {
        self.thread_id
    }

    pub fn protect_ptr<T>(&self, atomic_ptr: &AtomicPtr<T>) -> Option<NonNull<T>> {
        loop {
            let ptr = atomic_ptr.load(Ordering::Acquire);
            if ptr.is_null() {
                return None;
            }

            self.protect(ptr);

            if atomic_ptr.load(Ordering::Acquire) == ptr {
                return NonNull::new(ptr);
            }
        }
    }

    pub fn clear(&self) {
        let slot = &self.domain.inner.hazard_pointers[self.slot_index];
        slot.pointer
            .0
            .store(std::ptr::null_mut(), Ordering::Release);
    }
}

impl<'a> Drop for HazardPointer<'a> {
    fn drop(&mut self) {
        self.clear();
        let slot = &self.domain.inner.hazard_pointers[self.slot_index];
        slot.active.store(false, Ordering::Release);

        // Remove from thread's hazard indices
        if let Some(thread_data) = self.domain.find_thread_data(self.thread_id) {
            thread_data
                .hazard_indices
                .lock()
                .retain(|&idx| idx != self.slot_index);
        }
    }
}
