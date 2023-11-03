//! Implementation of [`PageTableEntry`] and [`PageTable`].

// use crate::task::current_user_token;

// use crate::task::get_current_task;

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, PhysAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
// use riscv::addr::page;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
/// 页表项
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    pub frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {    //页表
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {    // 获取用户空间传递的页表标识符
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    /// 获取用户空间传递的页表标识符，并构建一个对应的 PageTable 对象。
    /// 虚拟页号（vpn）查找页表项（PageTableEntry）。如果页表项不存在，则创建它。这个方法用于在页表中查找或创建多级页表的中间节点。
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    /// 根据虚拟页号（vpn）查找页表项，但不创建新的页表项。这个方法用于查找页表项是否已经存在，以决定是否可以进行映射或解除映射操作。
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }



    /// set the map between virtual page number and physical page number
    /// 建立虚拟页号（vpn）到物理页号（ppn）的映射，
    /// 设置相关的页表项标志（flags）。
    /// 将虚拟地址映射到物理地址。
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    /// 解除虚拟页号（vpn）到物理页号的映射
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    /// 从虚拟页号（vpn）获取页表项，
    /// 如果页表项存在，则返回它。
    /// 查找虚拟地址对应的物理地址或相关信息。
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the token from the page table
    /// 获取页表的标识符（token），
    /// 以便在需要时可以将它用于页表管理。
    /// 通常用于在地址空间切换时标识不同的页表。!!!!!!!!!!! 
    /// 
    /// 
    /// 终于找到你了，是不是就是你！！！！
    /// 
    /// 
    /// 按照 satp CSR 格式要求 构造一个无符号 64 位无符号整数，使得其 分页模式为 SV39，且将当前多级页表的根节点所在的物理页号填充进去
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// 参照上面的程序，进行转换
/// 参数参考上面的translate_byte_buffer, 这个函数参数是为 fs.rs::sys_write 服务的，其参数与 sys_write 相像
/// 我们应该是只需要一个地址就行
/// 好像也没有什么需要传进来的……
/// 找到page_table 调用其 translated 时发现需要一个VirtAddr参数
pub fn translated_va_to_pa(token: usize, virt_addr: VirtAddr) -> Option<PhysAddr> {
    let page_tabel = PageTable::from_token(token);
    if let Some(pte) = page_tabel.translate(virt_addr.clone().floor()) {
        let phys_addr_temp: PhysAddr = pte.ppn().into();
        let offset = virt_addr.clone().page_offset();
        Some((phys_addr_temp.0 + offset).into())
    } else {
        None
    }
}

// 获取当前任务page_table
// pub fn create_aaa(start: usize, len: usize, port: usize) {
//     let lifetime = 0;
//     let current_task = get_current_task(&lifetime);
//     current_task.memory_set.page_table.map(vpn, ppn, flags)
// }