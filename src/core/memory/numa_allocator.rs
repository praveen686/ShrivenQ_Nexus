use crate::core::memory::allocator::{AllocError, MemoryAllocator};
use crate::core::memory::lock_free_pool::{LockFreeMemoryPool, PoolConfig};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_os = "linux")]
use libc::{CPU_ISSET, CPU_SETSIZE, cpu_set_t, sched_getaffinity};

const DEFAULT_NUMA_NODES: usize = 2;
const CACHE_LINE_SIZE: usize = 64;

#[derive(Clone, Debug)]
pub struct NumaNode {
    pub id: usize,
    pub cpu_mask: Vec<usize>,
    pub memory_size: usize,
    pub distance_map: HashMap<usize, u8>,
}

#[derive(Clone, Debug)]
pub struct NumaConfig {
    pub nodes: Vec<NumaNode>,
    pub interleave: bool,
    pub local_alloc_preference: bool,
    pub migration_threshold: usize,
    pub pool_config: PoolConfig,
}

impl NumaConfig {
    #[cfg(target_os = "linux")]
    fn discover_numa_topology() -> Option<Vec<NumaNode>> {
        // Try to discover NUMA topology from /sys/devices/system/node/
        let node_dir = std::path::Path::new("/sys/devices/system/node");
        if !node_dir.exists() {
            return None;
        }

        let mut nodes = Vec::new();

        // Read available nodes
        let mut node_ids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(node_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("node") {
                    if let Ok(node_id) = name_str[4..].parse::<usize>() {
                        node_ids.push(node_id);
                    }
                }
            }
        }

        if node_ids.is_empty() {
            return None;
        }

        node_ids.sort();

        for &node_id in &node_ids {
            if let Some(node) = Self::read_numa_node_info(node_id) {
                nodes.push(node);
            }
        }

        if nodes.is_empty() { None } else { Some(nodes) }
    }

    #[cfg(target_os = "linux")]
    fn read_numa_node_info(node_id: usize) -> Option<NumaNode> {
        let node_path = format!("/sys/devices/system/node/node{}", node_id);

        // Read CPU list for this node
        let cpu_mask = Self::read_cpu_list(&format!("{}/cpulist", node_path))?;

        // Read memory size (try meminfo first, fallback to estimate)
        let memory_size = Self::read_memory_size(&format!("{}/meminfo", node_path))
            .unwrap_or(16 * 1024 * 1024 * 1024); // 16GB default

        // Create distance map - simplified for now
        let mut distance_map = HashMap::new();
        distance_map.insert(node_id, 10); // Local access

        Some(NumaNode {
            id: node_id,
            cpu_mask,
            memory_size,
            distance_map,
        })
    }

    #[cfg(target_os = "linux")]
    fn read_cpu_list(path: &str) -> Option<Vec<usize>> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut cpus = Vec::new();

        for part in content.trim().split(',') {
            if let Some(range_pos) = part.find('-') {
                // Range like "0-7"
                let start: usize = part[..range_pos].parse().ok()?;
                let end: usize = part[range_pos + 1..].parse().ok()?;
                for cpu in start..=end {
                    cpus.push(cpu);
                }
            } else {
                // Single CPU
                cpus.push(part.parse().ok()?);
            }
        }

        Some(cpus)
    }

    #[cfg(target_os = "linux")]
    fn read_memory_size(path: &str) -> Option<usize> {
        let content = std::fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if line.starts_with("Node") && line.contains("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let kb: usize = parts[2].parse().ok()?;
                    return Some(kb * 1024); // Convert KB to bytes
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn discover_numa_topology() -> Option<Vec<NumaNode>> {
        None
    }
}

impl Default for NumaConfig {
    fn default() -> Self {
        let nodes = if let Some(discovered_nodes) = Self::discover_numa_topology() {
            discovered_nodes
        } else {
            // Fallback to default configuration
            let mut nodes = Vec::new();
            for i in 0..DEFAULT_NUMA_NODES {
                let mut distance_map = HashMap::new();
                for j in 0..DEFAULT_NUMA_NODES {
                    distance_map.insert(j, if i == j { 10 } else { 20 });
                }

                nodes.push(NumaNode {
                    id: i,
                    cpu_mask: (i * 8..(i + 1) * 8).collect(),
                    memory_size: 32 * 1024 * 1024 * 1024,
                    distance_map,
                });
            }
            nodes
        };

        let mut pool_config = PoolConfig::default();
        // Ensure alignment is at least cache line size for NUMA
        pool_config.alignment = pool_config.alignment.max(CACHE_LINE_SIZE);

        Self {
            nodes,
            interleave: false,
            local_alloc_preference: true,
            migration_threshold: 1000,
            pool_config,
        }
    }
}

#[derive(Debug)]
pub struct NumaAllocator {
    config: NumaConfig,
    node_pools: Vec<Arc<LockFreeMemoryPool>>,
    current_node: AtomicUsize,
    allocation_stats: Arc<RwLock<NumaStats>>,
    thread_node_cache: Arc<RwLock<HashMap<std::thread::ThreadId, usize>>>,
}

#[derive(Default, Clone, Debug)]
pub struct NumaStats {
    pub allocations_per_node: HashMap<usize, usize>,
    pub cross_node_allocations: usize,
    pub local_allocations: usize,
    pub total_bytes_allocated: usize,
    pub node_stats: Vec<NodeStats>,
}

#[derive(Default, Clone, Debug)]
pub struct NodeStats {
    pub node_id: usize,
    pub allocations: usize,
    pub deallocations: usize,
    pub bytes_allocated: usize,
    pub bytes_freed: usize,
    pub allocated_chunks: usize,
    pub free_chunks: usize,
    pub total_memory: usize,
}

// Lightweight snapshot for external reporting - no cloning of large structures
#[derive(Debug)]
pub struct NumaStatsSnapshot {
    pub cross_node_allocations: usize,
    pub local_allocations: usize,
    pub total_bytes_allocated: usize,
    pub node_summaries: Vec<(usize, usize, usize, usize)>, // (node_id, allocated, free, total)
}

impl NumaAllocator {
    pub fn new(config: NumaConfig) -> Result<Self, AllocError> {
        let mut node_pools = Vec::new();
        let mut initial_stats = NumaStats::default();

        for node in &config.nodes {
            let mut pool_config = config.pool_config.clone();
            pool_config.max_chunks = node.memory_size / pool_config.chunk_size;

            let pool = LockFreeMemoryPool::new(pool_config)?;
            node_pools.push(Arc::new(pool));

            // Initialize stats for this node
            // Initialize allocation count for this node - previous value should be None
            if let Some(prev) = initial_stats.allocations_per_node.insert(node.id, 0) {
                tracing::warn!(
                    node_id = node.id,
                    prev_count = prev,
                    "Node allocation counter already existed"
                );
            }
            initial_stats.node_stats.push(NodeStats {
                node_id: node.id,
                allocations: 0,
                deallocations: 0,
                bytes_allocated: 0,
                bytes_freed: 0,
                allocated_chunks: 0,
                free_chunks: 0,
                total_memory: 0,
            });
        }

        Ok(Self {
            config,
            node_pools,
            current_node: AtomicUsize::new(0),
            allocation_stats: Arc::new(RwLock::new(initial_stats)),
            thread_node_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn get_current_numa_node(&self) -> usize {
        #[cfg(target_os = "linux")]
        {
            if let Some(node) = self.get_linux_numa_node() {
                return node;
            }
        }

        if let Some(node) = self.get_cached_thread_node() {
            return node;
        }

        let thread_id = std::thread::current().id();
        let hash = self.hash_thread_id(thread_id);
        let node = hash % self.config.nodes.len();

        self.cache_thread_node(thread_id, node);
        node
    }

    #[cfg(target_os = "linux")]
    fn get_linux_numa_node(&self) -> Option<usize> {
        unsafe {
            let mut cpu_set: cpu_set_t = std::mem::zeroed();

            if sched_getaffinity(0, std::mem::size_of::<cpu_set_t>(), &mut cpu_set) == 0 {
                for cpu in 0..CPU_SETSIZE as usize {
                    if CPU_ISSET(cpu, &cpu_set) {
                        for node in &self.config.nodes {
                            if node.cpu_mask.contains(&cpu) {
                                return Some(node.id);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn get_linux_numa_node(&self) -> Option<usize> {
        None
    }

    fn get_cached_thread_node(&self) -> Option<usize> {
        let thread_id = std::thread::current().id();
        self.thread_node_cache.read().get(&thread_id).copied()
    }

    fn cache_thread_node(&self, thread_id: std::thread::ThreadId, node: usize) {
        // Cache the node for this thread - update if already exists
        if let Some(prev_node) = self.thread_node_cache.write().insert(thread_id, node) {
            tracing::debug!(thread_id = ?thread_id, prev_node, new_node = node,
                          "Updated thread NUMA node assignment");
        }
    }

    fn hash_thread_id(&self, id: std::thread::ThreadId) -> usize {
        let id_bytes = format!("{:?}", id).into_bytes();
        let mut hash: usize = 0;
        for byte in id_bytes {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        hash
    }

    fn select_allocation_node(&self) -> usize {
        if self.config.interleave {
            self.current_node.fetch_add(1, Ordering::Relaxed) % self.config.nodes.len()
        } else if self.config.local_alloc_preference {
            self.get_current_numa_node()
        } else {
            0
        }
    }

    fn try_allocate_from_node(
        &self,
        node_id: usize,
        layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        if node_id >= self.node_pools.len() {
            return Err(AllocError::NumaNodeUnavailable(node_id));
        }

        self.node_pools[node_id].allocate(layout)
    }

    fn update_stats(&self, node_id: usize, size: usize, is_local: bool) {
        let mut stats = self.allocation_stats.write();
        *stats.allocations_per_node.entry(node_id).or_insert(0) += 1;
        stats.total_bytes_allocated += size;

        if is_local {
            stats.local_allocations += 1;
        } else {
            stats.cross_node_allocations += 1;
        }
    }

    pub fn allocate_on_node(
        &self,
        node_id: usize,
        layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        let current_node = self.get_current_numa_node();
        let is_local = node_id == current_node;

        let result = self.try_allocate_from_node(node_id, layout)?;
        self.update_stats(node_id, layout.size(), is_local);

        Ok(result)
    }

    pub fn get_node_distance(&self, from: usize, to: usize) -> Option<u8> {
        self.config
            .nodes
            .get(from)
            .and_then(|node| node.distance_map.get(&to))
            .copied()
    }

    pub fn with_stats<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&NumaStats) -> R,
    {
        let stats_guard = self.allocation_stats.read();
        f(&*stats_guard)
    }

    pub fn get_stats_snapshot(&self) -> NumaStatsSnapshot {
        let stats_guard = self.allocation_stats.read();

        // Only copy the minimal data needed for reporting
        let mut node_summaries = Vec::with_capacity(self.node_pools.len());

        for (node_id, pool) in self.node_pools.iter().enumerate() {
            let pool_stats = pool.get_stats();
            node_summaries.push((
                node_id,
                pool_stats.allocated_chunks,
                pool_stats.free_chunks,
                pool_stats.total_memory_bytes,
            ));
        }

        NumaStatsSnapshot {
            cross_node_allocations: stats_guard.cross_node_allocations,
            local_allocations: stats_guard.local_allocations,
            total_bytes_allocated: stats_guard.total_bytes_allocated,
            node_summaries,
        }
    }
}

impl MemoryAllocator for NumaAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let preferred_node = self.select_allocation_node();

        match self.try_allocate_from_node(preferred_node, layout) {
            Ok(ptr) => {
                self.update_stats(preferred_node, layout.size(), true);
                Ok(ptr)
            }
            Err(_) => {
                for (node_id, pool) in self.node_pools.iter().enumerate() {
                    if node_id != preferred_node {
                        if let Ok(ptr) = pool.allocate(layout) {
                            self.update_stats(node_id, layout.size(), false);
                            return Ok(ptr);
                        }
                    }
                }
                Err(AllocError::OutOfMemory)
            }
        }
    }

    fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        for pool in &self.node_pools {
            pool.deallocate(ptr, layout);
            return;
        }
    }

    fn available_memory(&self) -> usize {
        self.node_pools
            .iter()
            .map(|pool| pool.available_memory())
            .sum()
    }

    fn total_memory(&self) -> usize {
        self.node_pools.iter().map(|pool| pool.total_memory()).sum()
    }
}
