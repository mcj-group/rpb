use std::collections::BinaryHeap;
// ============================================================================
// This code is part of RPB.
// ----------------------------------------------------------------------------
// MIT License
//
// Copyright (c) 2023-present Javad Abdi, Mark C. Jeffrey
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// ============================================================================

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use rand::{Rng, thread_rng};

use crate::util::Padded;


const C: usize = 4;


pub struct MultiQueue<PQElem> {
    pq_list: Vec::<Padded<Mutex<BinaryHeap<PQElem>>>>,
    pq_list_size: usize,
    num_empty: AtomicU32,
}

impl<PQElem: Ord + Copy> MultiQueue<PQElem> {
    pub fn new(num_threads: usize) -> Self {
        assert!(num_threads > 0);
        let pq_list_size = num_threads * C;
        Self {
            pq_list: (0..pq_list_size)
                .map(|_| Padded::new(Mutex::new(BinaryHeap::new())))
                .collect::<Vec<_>>(),
            pq_list_size,
            num_empty: AtomicU32::new(pq_list_size as u32),
        }
    }

    fn lock_a_queue(&self) -> (MutexGuard<'_, BinaryHeap<PQElem>>, usize) {
        let mut index;
        let q = loop {
            index = thread_rng().gen_range(0..self.pq_list_size);
            if let Ok(pq) = self.pq_list[index].try_lock() {
                break pq;
            }
        };
        (q, index)
    }

    fn lock_a_queue_except(&self, except: usize) -> (MutexGuard<'_, BinaryHeap<PQElem>>, usize) {
        let mut index;
        let q = loop {
            index = thread_rng().gen_range(0..self.pq_list_size);
            if index == except { continue; }
            else if let Ok(pq) = self.pq_list[index].try_lock() {
                break pq;
            }
        };
        (q, index)
    }

    pub fn push(&self, elem: PQElem) {
        let (mut pq, _) = self.lock_a_queue();
        if pq.is_empty() {
            self.num_empty.fetch_sub(1, Ordering::Relaxed);
        }
        pq.push(elem);
    }

    // TODO: Mark suggested trying ray's pop.
    pub fn pop(&self) -> Option<PQElem> {
        loop {
            let (mut val_1, mut val_2) = unsafe { (
                std::mem::MaybeUninit::<PQElem>::uninit().assume_init(),
                std::mem::MaybeUninit::<PQElem>::uninit().assume_init()
            ) };

            let (q_1, idx_1) = self.lock_a_queue();
            let empty_1 = q_1.is_empty();
            if !empty_1 { val_1 = *q_1.peek().unwrap(); };
            drop(q_1);

            let (q_2, idx_2) = self.lock_a_queue_except(idx_1);
            let empty_2 = q_2.is_empty();
            if !empty_2 { val_2 = *q_2.peek().unwrap(); };
            drop(q_2);

            let selected;
            if empty_1 && empty_2 {
                if self.num_empty.load(Ordering::Relaxed)
                    == self.pq_list_size as u32 { return None; }
                else { continue; }
            } else if !empty_1 {
                if !empty_2 {
                    selected = if val_1 > val_2 { idx_1 }
                    else { idx_2 }
                } else { selected = idx_1; }
            } else { selected = idx_2; }

            let mut q = self.pq_list[selected].lock().unwrap();
            if let Some(ret) = q.pop() {
                if q.is_empty() {
                    self.num_empty.fetch_add(1, Ordering::Relaxed);
                }
                return Some(ret);
            } else { continue; }
        }
    }
}


#[cfg(test)]
mod multiqueue_tests {
    use super::*;

    #[test]
    fn single_thread() {
        let pq = MultiQueue::<u32>::new(1);
        pq.push(1);
        pq.push(3);
        pq.push(2);

        let mut rets = vec![];
        for _ in 0..1000 {
            if let Some(elem) = pq.pop() {
                rets.push(elem);
            }
        }
        rets.sort();
        assert_eq!(rets, vec![1, 2, 3]);
    }

    #[test]
    fn multi_thread() {
        let pq = MultiQueue::<u32>::new(2);

        std::thread::scope(|s| {
            s.spawn(|| {
                pq.push(1);
                pq.push(3);
                pq.push(2);
            });
            s.spawn(|| {
                pq.push(4);
                pq.push(6);
                pq.push(5);
            });
        });

        let mut rets1 = vec![];
        let mut rets2 = vec![];
        std::thread::scope(|s| {
            s.spawn(|| {
                for _ in 0..1000 {
                    if let Some(elem) = pq.pop() {
                        rets1.push(elem);
                    }
                }
            });
            s.spawn(|| {
                for _ in 0..1000 {
                    if let Some(elem) = pq.pop() {
                        rets2.push(elem);
                    }
                }
            });
        });

        rets1.append(&mut rets2);
        rets1.sort();
        assert_eq!(rets1, vec![1, 2, 3, 4, 5, 6]);
    }
}
