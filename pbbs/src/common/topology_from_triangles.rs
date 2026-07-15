use std::cmp::Ordering;
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

use std::marker::PhantomData;
use rayon::prelude::*;

use parlay::make_mut;
use parlay::hash_table::*;
use parlay::utilities::hash64;
use crate::common::{
    atomics::atomic_cas,
    geometry::{Triangles, Point2d},
    topology::{Vertex, Triangle, NULL_TRI}
};


type Idx = i32;
type IdxPair = (Idx, Idx);
type Vtx<'a> = Vertex<'a>;
type Tri<'a> = Triangle<'a>;
type Edge<'a> = (IdxPair, &'a Tri<'a>);

static NULL_EDGE: Edge = ((0, 0), &NULL_TRI);


struct HashEdges<'a> { pahntom: PhantomData<&'a bool> }

impl<'a> HashHelper for HashEdges<'a> {
    type KT = IdxPair;
    type ET = &'a Edge<'a>;

    fn empty    () -> Self::ET { &NULL_EDGE }
    fn get_key  (v: Self::ET) -> Self::KT { v.0 }
    fn replace_q(_s: Self::ET, _s2: Self::ET) -> bool { false }

    fn hash     (s: Self::KT) -> usize {
        hash64(s.0 as u64).overflowing_add(
            hash64(s.1 as u64).overflowing_mul(3).0
        ).0 as usize
    }

    #[inline(always)]
    fn is_empty (s: &Self::ET) -> bool {
        (*s) as *const Edge == &NULL_EDGE as *const Edge
    }

    fn cmp (s1: Self::KT, s2:Self::KT) -> Ordering {
        if s1.0 > s2.0      { Ordering::Greater }
        else if s1.0 < s2.0 { Ordering::Less }
        else if s1.1 > s2.1 { Ordering::Greater }
        else if s1.1 < s2.1 { Ordering::Less }
        else                { Ordering::Equal }
    }

    fn cas (p: &mut Self::ET, o: Self::ET, n: Self::ET) -> bool {
        atomic_cas(p, o, n)
    }
}


type EdgeTable<'a> = HashTable<HashEdges<'a>>;

pub fn topology_from_triangles(
    tris: &Triangles<Point2d<f64>>,
    extra_points: usize
) -> (Vec<Tri<'_>>, Vec<Vtx<'_>>) {
    let (n, m) = (tris.num_points(), tris.num_triangles());
    let vs: Vec<_> = (0..n+extra_points).into_par_iter().map(
        |i| if i<n {Vtx::new(tris.p[i], i)} else {Vtx::default()}
    ).collect();
    let et = EdgeTable::new(m * 6, 1.5);
    let mut es: Vec<Edge> = Vec::with_capacity(m * 3);
    let mut triangs: Vec<Tri> = Vec::with_capacity(m + 2 * extra_points);
    unsafe {
        es.set_len(m * 3);
        triangs.set_len(m + 2 * extra_points);
    }
    let es_ptr = es.as_ptr() as usize;
    triangs[..m].par_iter().enumerate().for_each(|(i, t)| {
        for j in 0..3 {
            unsafe {
                *(es_ptr as *mut Edge).add(3 * i + j) =
                    ((tris.t[i][j] as Idx, tris.t[i][(j + 1) % 3] as Idx), t);
                et.insert(& *(es_ptr as *const Edge).add(3 * i + j));
                make_mut!(t, Tri).unwrap().vtx[(j + 2) % 3] =
                    Some(&vs[tris.t[i][j] as usize]);
            }
        }
    });

    triangs[..m].par_iter_mut().enumerate().for_each(|(i, ti)| {
        ti.id = i; ti.initialized = true;
        ti.bad = 0;
        for j in 0..3 {
            let key = (tris.t[i][(j + 1) % 3] as Idx, tris.t[i][j] as Idx);
            ti.ngh[j] = if let Some(ed) = et.find(key) { Some(ed.1) }
            else { None }
        }
    });

    (triangs, vs)
}
