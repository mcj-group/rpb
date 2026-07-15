
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

use std::default::Default;

use crate::common::geometry::{Point2d, in_circle, counter_clock_wise, angle};
use parlay::make_mut;


type Tri<'a> = Triangle<'a>;
type Vtx<'a> = Vertex<'a>;

#[derive(Clone, Copy)]
pub struct Vertex<'a> {
    pub pt: Point2d<f64>,
    pub id: i32,
    pub reserve: i32,
    pub t: Option<&'a Tri<'a>>,
    pub bad_t: Option<&'a Tri<'a>>,
}

#[derive(Clone, Copy)]
pub struct Triangle<'a> {
    pub id: usize,
    pub vtx: [Option<&'a Vtx<'a>>; 3],
    pub ngh: [Option<&'a Tri<'a>>; 3],
    pub initialized: bool,
    pub bad: u8, // used to mark badly shaped triangles.
}

#[derive(Clone, Copy)]
pub struct SimpleX<'a> {
    pub o: i32,
    pub boundary: bool,
    pub t: Option<&'a Tri<'a>>,
}


#[inline(always)]
fn mod3(i: i32) -> i32 { if i > 2 { i-3 } else { i } }

impl<'a> Vertex<'a> {
    pub fn new(p: Point2d<f64>, i: usize) -> Self {
        Self { pt: p, id: i as i32, reserve: -1, t: None, bad_t: None }
    }

    pub fn print(&self) {
        println!("{}({},{})", self.id, self.pt.x, self.pt.y);
    }
}

impl<'a> Default for Vertex<'a> {
    fn default() -> Self {
        Self { pt: Point2d::default(), id: 0, reserve: -1, t: None, bad_t: None }
    }
}

impl<'a> Triangle<'a> {
    pub fn set_t(
        &mut self,
        t1: Option<&'a Tri>,
        t2: Option<&'a Tri>,
        t3: Option<&'a Tri>
    ) {
        self.ngh[0] = t1; self.ngh[1] = t2; self.ngh[2] = t3;
    }

    pub fn set_v(&mut self, v1: &'a Vtx, v2: &'a Vtx, v3: &'a Vtx) {
        self.vtx[0] = Some(v1);
        self.vtx[1] = Some(v2);
        self.vtx[2] = Some(v3);
    }

    pub fn locate(&self, t: &Tri) -> i32 {
        for i in 0..3 {
            if let Some(ngh) = self.ngh[i] {
                if ngh as *const Tri == t as *const Tri {
                    return i as i32;
                }
            }
        }
        panic!("did not locate back pointer in triangulation\n");
    }

    pub fn update(&mut self, t: &Tri, tn: &'a Tri) {
        for i in 0..3 {
            if let Some(ngh) = self.ngh[i]{
                if ngh as *const Tri == t as *const Tri {
                    self.ngh[i] = Some(tn);
                    return;
                }
            }
        }
        panic!("triangle:update: did not found the old neighbor.");
    }
}

pub static NULL_TRI: Triangle = Triangle {
    id: 0,
    vtx: [None, None, None],
    ngh: [None, None, None],
    initialized: false,
    bad: 0
};

impl<'a> Default for Triangle<'a> {
    fn default() -> Self {
        Self {
            id: 0,
            vtx: [None, None, None],
            ngh: [None, None, None],
            initialized: false,
            bad: 0
        }
    }
}

impl<'a> SimpleX<'a>
{
    pub fn new(t: &'a Tri, o: i32) -> Self {
        Self { o, boundary: false, t: Some(t) }
    }

    pub fn new_from_vtx(_v1: &Vtx, _v2: &Vtx, _v3: &Vtx, _t: &'a Tri) {
        todo!()
    }

    pub fn enable_bound(mut self) -> Self {
        self.boundary = true;
        self
    }

    pub fn valid(&self)       -> bool { !self.boundary }
    pub fn is_triangle(&self) -> bool { !self.boundary }
    pub fn is_boundary(&self) -> bool { self.boundary }

    pub fn rotate(&self) -> Self {
        Self::new(self.t.unwrap(), mod3(self.o+1))
    }

    pub fn across(&self) -> Self {
        let to = self.t.unwrap().ngh[self.o as usize];
        if let Some(to) = to {
            Self::new(to, to.locate(self.t.unwrap()))
        } else {
            Self::new(self.t.unwrap(), self.o).enable_bound()
        }
    }

    pub fn first_vertex(&self) -> &'a Vtx<'_> {
        self.t.unwrap().vtx[self.o as usize].unwrap()
    }

    pub fn in_circ(&self, v: &Vtx<'a>) -> bool {
        if self.boundary || self.t.is_none() {
            false
        } else {
            let tv = &self.t.unwrap().vtx;
            in_circle(
                tv[0].unwrap().pt,
                tv[1].unwrap().pt,
                tv[2].unwrap().pt,
                v.pt
            )
        }
    }

    pub fn far_angle(&self) -> f64 {
        let tv = &self.t.unwrap().vtx;
        angle(
            tv[mod3(self.o+1) as usize].unwrap().pt,
            tv[self.o as usize].unwrap().pt,
            tv[mod3(self.o+2) as usize].unwrap().pt
        )
    }

    pub fn outside(&self, v: &Vtx) -> bool {
        if self.boundary || self.t.is_none() {
            false
        } else {
            let tv = &self.t.unwrap().vtx;
            counter_clock_wise(
                tv[mod3(self.o + 2) as usize].unwrap().pt,
                v.pt, tv[self.o as usize].unwrap().pt
            )
        }
    }

    pub fn flip(&self) {
        let s = self.across();
        let st = s.t.unwrap();
        let t = self.t.unwrap();
        let os1 = mod3(s.o+1) as usize;
        let o1 = mod3(self.o+1) as usize;

        // JA: Let's do it all unsafely.
        let t1 = t.ngh[o1];
        let t2 = st.ngh[os1];
        let v1 = t.vtx[o1];
        let v2 = st.vtx[os1];

        unsafe {
            let t = make_mut!(t, Tri).unwrap();
            make_mut!(t.vtx[self.o as usize].unwrap(), Vtx).unwrap().t = s.t;
            t.vtx[self.o as usize] = v2;
            t.ngh[self.o as usize] = t2;
            if let Some(t2) = t2 {
                make_mut!(t2, Tri).unwrap().update(st, t);
            }
            t.ngh[o1] = s.t;

            let st = make_mut!(st, Tri).unwrap();
            make_mut!(st.vtx[s.o as usize].unwrap(), Vtx).unwrap().t = self.t;
            st.vtx[s.o as usize] = v1;
            st.ngh[s.o as usize] = t1;
            if let Some(t1) = t1 {
                make_mut!(t1, Tri).unwrap().update(t, st)
            }
            st.ngh[os1] = self.t;
        }
    }

    pub fn split(&self, v: &'a Vtx, ta0: &'a Tri, ta1: &'a Tri) {
        let t = self.t.unwrap();
        unsafe { make_mut!(v, Vtx).unwrap().t = self.t; }

        let (_t1, t2, t3) = (t.ngh[0], t.ngh[1], t.ngh[2]);
        let (v1, v2, v3) = (
            t.vtx[0].unwrap(),
            t.vtx[1].unwrap(),
            t.vtx[2].unwrap()
        );

        unsafe {
            let t = make_mut!(t, Tri).unwrap();
            t.ngh[1] = Some(ta0);
            t.ngh[2] = Some(ta1);
            t.vtx[1] = Some(v);

            let ta0 = make_mut!(ta0, Tri).unwrap();
            ta0.set_t(t2, Some(ta1), Some(t));
            ta0.set_v(v2, v, v1);

            let ta1 = make_mut!(ta1, Tri).unwrap();
            ta1.set_t(t3, Some(t), Some(ta0));
            ta1.set_v(v3, v, v2);

            if let Some(t2) = t2 { make_mut!(t2, Tri).unwrap().update(t, ta0) }
            if let Some(t3) = t3 { make_mut!(t3, Tri).unwrap().update(t, ta1) }
            make_mut!(v2, Vtx).unwrap().t = Some(ta0);
        }
    }

    pub fn split_boundary(&self, v: &Vtx, ta: &Tri) {
        let o1 = mod3(self.o+1) as usize;
        let o2 = mod3(self.o+2) as usize;
        let t = self.t.unwrap();
        if let Some(_) = t.ngh[self.o as usize] {
            panic!("simplex::splitBoundary: not boundary");
        }
        unsafe { make_mut!(v, Vtx).unwrap().t = self.t; }
        let t2 = t.ngh[o2];
        let (v1, v2) = (t.vtx[o1].unwrap(), t.vtx[o2].unwrap());

        unsafe {
            let t = make_mut!(t, Tri).unwrap();
            t.ngh[o2] = Some(ta);
            t.vtx[o2] = Some(v);
            let ta = make_mut!(ta, Tri).unwrap();
            ta.set_t(t2, None, Some(t)); ta.set_v(v2, v, v1);
            if let Some(t2) = t2 { make_mut!(t2, Tri).unwrap().update(t, ta); }
            make_mut!(v2, Vtx).unwrap().t = self.t;
        }
    }

    pub fn extend(self, v: &'a Vtx<'a>, ta: &'a mut Tri<'a>) -> Self {
        let t = self.t.unwrap();
        if let Some(_) = t.ngh[self.o as usize] {
            panic!("simplex::extend: not boundary");
        }
        unsafe { make_mut!(t, Tri).unwrap().ngh[self.o as usize] = Some(ta); }
        ta.set_v(
            t.vtx[self.o as usize].unwrap(),
            t.vtx[mod3(self.o+2) as usize].unwrap(),
            v
        );
        ta.set_t(None, self.t, None);
        unsafe { make_mut!(v, Vtx).unwrap().t = Some(ta); }
        Self::new(ta, 0)
    }
}

impl<'a> Default for SimpleX<'a> {
    fn default() -> Self {
        Self { o: 0, boundary: false, t: None }
    }
}

impl<'a> SimpleX<'a> {
    pub fn print(&self) {
        if let Some(t) = self.t {
            print!("vtxs=");
            for i in 0..3 {
                if let Some(v) = t.vtx[mod3(i+self.o) as usize] {
                    println!("{}({},{}) ", v.id, v.pt.x, v.pt.y);
                } else { println!("NULL ") }
            }
        } else { println!("NULL simp"); }
    }
}
