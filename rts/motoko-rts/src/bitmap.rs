use crate::alloc::alloc_blob;
// use crate::gc::get_heap_base;
use crate::mem::memzero;
use crate::print::*;
use crate::types::{size_of, Blob, Bytes, Obj, WORD_SIZE};

/// Current bitmap
static mut BITMAP_PTR: *mut u8 = core::ptr::null_mut();

pub unsafe fn alloc_bitmap(heap_size: Bytes<u32>) {
    // We will have at most this many objects in the heap, each requiring a bit
    let n_bits = heap_size.to_words().0;
    // Each byte will hold 8 bits.
    let bitmap_bytes = (n_bits + 7) / 8;
    // Also round allocation up to 8-bytes to make iteration efficient. We want to be able to read
    // 64 bits in a single read and check as many bits as possible with a single `word != 0`.
    let bitmap_bytes = Bytes(((bitmap_bytes + 7) / 8) * 8);
    // Allocating an actual object here as otherwise dump_heap gets confused
    let blob = alloc_blob(bitmap_bytes).unskew() as *mut Blob;
    memzero(blob.payload_addr() as usize, bitmap_bytes.to_words());

    BITMAP_PTR = blob.payload_addr()
}

pub unsafe fn free_bitmap() {
    BITMAP_PTR = core::ptr::null_mut();
}

pub unsafe fn get_bit(idx: u32) -> bool {
    let byte_idx = idx / 8;
    let byte = *BITMAP_PTR.add(byte_idx as usize);
    let bit_idx = idx % 8;
    (byte >> bit_idx) & 0b1 == 0b1
}

pub unsafe fn set_bit(idx: u32) {
    let byte_idx = idx / 8;
    let byte = *BITMAP_PTR.add(byte_idx as usize);
    let bit_idx = idx % 8;
    let new_byte = byte | (0b1 << bit_idx);
    *BITMAP_PTR.add(byte_idx as usize) = new_byte;
}

struct BitmapIterState {
    /// Size of the bitmap, in 64-bit words words. Does not change after initialization.
    size: u32,
    /// Current 64-bit word index
    current_word_idx: u32,
    /// Current 64-bit word in the bitmap that we're iterating. We read in 64-bit chunks to be able
    /// to check as many bits as possible with a single `word != 0`.
    current_word: u64,
    /// Bits left in the current 64-bit word. Used to compute index of a bit in the bitmap. We
    /// can't use a global index here as we don't know how much to bump it when `current_word` is
    /// 0 and we move to the next 64-bit word.
    bits_left: u32,
}

// Iterates set bits
struct BitmapIter {
    state: BitmapIterState,
}

// Iterates unset bits
struct BitmapUnsetIter {
    state: BitmapIterState,
}

pub unsafe fn iter_bits() -> impl Iterator<Item = u32> {
    let blob_len_bytes = (BITMAP_PTR.sub(size_of::<Blob>().to_bytes().0 as usize) as *mut Obj)
        .as_blob()
        .len()
        .0;

    debug_assert_eq!(blob_len_bytes % 8, 0);

    let blob_len_64bit_words = blob_len_bytes / 8;

    let current_word = if blob_len_64bit_words == 0 {
        0
    } else {
        *(BITMAP_PTR as *const u64)
    };

    BitmapIter {
        state: BitmapIterState {
            size: blob_len_64bit_words,
            current_word_idx: 0,
            current_word,
            bits_left: 64,
        },
    }
}

pub unsafe fn iter_unset_bits() -> impl Iterator<Item = u32> {
    let blob_len_bytes = (BITMAP_PTR.sub(size_of::<Blob>().to_bytes().0 as usize) as *mut Obj)
        .as_blob()
        .len()
        .0;

    debug_assert_eq!(blob_len_bytes % 8, 0);

    let blob_len_64bit_words = blob_len_bytes / 8;

    let current_word = if blob_len_64bit_words == 0 {
        0
    } else {
        *(BITMAP_PTR as *const u64)
    };

    BitmapUnsetIter {
        state: BitmapIterState {
            size: blob_len_64bit_words,
            current_word_idx: 0,
            current_word,
            bits_left: 64,
        },
    }
}

impl Iterator for BitmapIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        debug_assert!(self.state.current_word_idx <= self.state.size);

        // Outer loop iterates 64-bit words
        loop {
            if self.state.current_word == 0 && self.state.current_word_idx == self.state.size {
                return None;
            }

            // Inner loop iterates bits in the current word
            while self.state.current_word != 0 {
                if self.state.current_word & 0b1 == 0b1 {
                    let bit_idx = (self.state.current_word_idx * 64) + (64 - self.state.bits_left);
                    self.state.current_word >>= 1;
                    self.state.bits_left -= 1;
                    return Some(bit_idx);
                } else {
                    let shift_amt = self.state.current_word.trailing_zeros();
                    self.state.current_word >>= shift_amt;
                    self.state.bits_left -= shift_amt;
                }
            }

            // Move on to next word
            self.state.current_word_idx += 1;
            if self.state.current_word_idx == self.state.size {
                return None;
            }
            self.state.current_word =
                unsafe { *(BITMAP_PTR as *const u64).add(self.state.current_word_idx as usize) };
            self.state.bits_left = 64;
        }
    }
}

impl Iterator for BitmapUnsetIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        debug_assert!(self.state.current_word_idx <= self.state.size);

        // Outer loop iterates 64-bit words
        loop {
            if self.state.current_word_idx == self.state.size {
                return None;
            }

            // Inner loop iterates bits in the current word
            while self.state.current_word != u64::MAX && self.state.bits_left != 0 {
                if self.state.current_word & 0b1 == 0b0 {
                    let bit_idx = (self.state.current_word_idx * 64) + (64 - self.state.bits_left);
                    self.state.current_word >>= 1;
                    self.state.bits_left -= 1;
                    return Some(bit_idx);
                } else {
                    let shift_amt = self.state.current_word.trailing_ones();
                    self.state.current_word >>= shift_amt;
                    self.state.bits_left -= shift_amt;
                }
            }

            // Move on to next word
            self.state.current_word_idx += 1;
            if self.state.current_word_idx == self.state.size {
                return None;
            }
            self.state.current_word =
                unsafe { *(BITMAP_PTR as *const u64).add(self.state.current_word_idx as usize) };
            self.state.bits_left = 64;
        }
    }
}

// pub unsafe fn print_bitmap() {
//     let heap_base = get_heap_base();
// 
//     println!(50, "Bitmap:");
//     for bit_idx in iter_bits() {
//         println!(150, "{} ({:#x})", bit_idx, bit_idx * WORD_SIZE + heap_base);
//     }
//     println!(50, "End of bitmap");
// }
