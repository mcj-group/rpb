use std::{ops::Index, str::FromStr};
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

use rayon::prelude::*;

use crate::{DefInt, DefFloat};

// **************************************************************
//    EDGE ARRAY REPRESENTATION
// **************************************************************

#[derive(Clone, Copy)]
pub struct Edge {
    pub u: DefInt,
    pub v: DefInt,
}

impl Edge {
    pub fn new(u: DefInt, v: DefInt) -> Self { Self { u, v } }
}

impl Default for Edge { fn default() -> Self { Self { u: 0, v: 0 } } }

impl FromStr for Edge {
    type Err = ParseEdgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: Vec<&str> = s
            .trim()
            .split_whitespace()
            .collect();
        if s.len() != 2 { return Err(ParseEdgeError); }
        let (a, b) = (s[0].parse(), s[1].parse());
        if a.is_err() || b.is_err() { return Err(ParseEdgeError); }
        Ok(Self::new(a.unwrap(), b.unwrap()))
    }
}

pub struct ParseEdgeError;

impl std::fmt::Display for ParseEdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Can not parse as edge.")
    }
}

impl std::fmt::Debug for ParseEdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{{ file: {}, line: {} }}: can not parse as edge.", file!(), line!())
    }
}

pub struct EdgeArray {
    pub es: Vec<Edge>,
    pub num_rows: usize,
    pub num_cols: usize,
    pub non_zeros: usize,
}

impl EdgeArray {
    pub fn new(es: Vec<Edge>, r: usize, c: usize) -> Self {
        Self {
            non_zeros: es.len(),
            es,
            num_rows: r,
            num_cols: c
        }
    }
}

impl Index<usize> for EdgeArray {
    type Output = Edge;

    fn index(&self, index: usize) -> &Self::Output {
        #[cfg(not(feature = "mem_safe"))]
        unsafe { self.es.as_ptr().add(index).as_ref().unwrap() }
        #[cfg(feature = "mem_safe")]
        &self.es[index]
    }
}

// **************************************************************
//    WEIGHED EDGE ARRAY
// **************************************************************

#[derive(Clone, Copy)]
pub struct WghEdge {
    pub u: DefInt,
    pub v: DefInt,
    pub w: DefFloat,
}

impl WghEdge {
    pub fn new(u: DefInt, v: DefInt, w: DefFloat) -> Self
    { Self { u, v, w } }
}

impl Default for WghEdge {
    fn default() -> Self { Self { u: 0, v: 0, w: 0.0 } }
}

impl FromStr for WghEdge {
    type Err = ParseEdgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: Vec<&str> = s.trim().split_whitespace().collect();
        if s.len() != 3 { return Err(ParseEdgeError); }
        let (a, b, w) = (s[0].parse(), s[1].parse(), s[2].parse());
        if a.is_err() || b.is_err() || w.is_err() {
            return Err(ParseEdgeError);
        }
        Ok(Self::new(a.unwrap(), b.unwrap(), w.unwrap()))
    }
}

#[derive(Clone)]
pub struct WghEdgeArray {
    pub es: Vec<WghEdge>,
    pub n: usize,
    pub m: usize,
}

impl WghEdgeArray {
    pub fn new(es: Vec<WghEdge>, n: usize) -> Self {
        let es_len = es.len();
        Self { es, n, m: es_len }
    }

    pub fn get_mut(&mut self, index: usize) -> &mut WghEdge {
        &mut self.es[index]
    }
}

impl Index<usize> for WghEdgeArray {
    type Output = WghEdge;

    fn index(&self, index: usize) -> &Self::Output {
        &self.es[index]
    }
}


// **************************************************************
//    ADJACENCY ARRAY REPRESENTATION
// **************************************************************

pub struct Vertex<'a> {
    pub neighbors: &'a[DefInt],
    pub degree: usize,
}

impl<'a> Vertex<'a> {
    pub fn new(n: &'a[DefInt], d: usize) -> Self {
        Self { neighbors: n, degree: d }
    }
}

impl<'a> Default for Vertex<'a> {
    fn default() -> Self {
        Self { neighbors: &[], degree: 0 }
    }
}

pub struct Graph {
    pub offsets: Vec<DefInt>,
    pub edges: Vec<DefInt>,
    pub degrees: Vec<DefInt>,
    pub n: usize,
    pub m: usize,
}

impl Graph {
    pub const fn num_vertices(&self) -> usize
    { self.n }

    pub fn num_edges(&self) -> usize {
        if self.degrees.len() == 0 { self.m }
        else {
            todo!("not yet implemented!");
        }
    }

    pub const fn get_offsets(&self) -> &Vec<DefInt>
    { &self.offsets }

    pub fn add_degrees(&mut self) {
        debug_assert!(self.degrees.len() == 0);
        self.degrees = (0..self.n)
            .into_par_iter()
            .map(|i| self.offsets[i+1] - self.offsets[i])
            .collect();
    }

    pub fn new(offsets: &[DefInt], edges: &[DefInt], n: usize) -> Self
    {
        debug_assert_eq!(n + 1, offsets.len());
        debug_assert_eq!(edges.len(), offsets[n] as usize);

        Self {
            offsets: offsets.to_vec(),
            edges: edges.to_vec(),
            n: n,
            m: edges.len(),
            degrees: vec![],
        }
    }

    #[inline(always)]
    pub fn index(&self, i: usize) -> Vertex<'_> {
        debug_assert!(i < self.n);

        let (of, of_next) = (
            self.offsets[i] as usize,
            self.offsets[i+1] as usize
        );

        let d = match self.degrees.len() {
            0 => of_next - of,
            _ => self.degrees[i] as usize,
        };
        let n = &self.edges[of..of_next];

        Vertex { neighbors: n, degree: d, }
    }
}
