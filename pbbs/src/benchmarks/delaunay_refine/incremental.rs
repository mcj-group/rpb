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

use parlay::hash_table::*;
use parlay::utilities::hash64;
use parlay::{make_mut, Timer};
use parlay::primitives::{pack, nc_pack, pack_index};
use crate::common::topology::*;
use crate::common::geometry::*;
use crate::common::atomics::{write_max_i32, atomic_cas};
use crate::common::topology_from_triangles::topology_from_triangles;


type P = Point2d<f64>;
type Tri<'a> = Triangle<'a>;
type Vtx<'a> = Vertex<'a>;
type Spx<'a> = SimpleX<'a>;


struct Qs<'a> {
    vertex_q: Vec<usize>,
    simplex_q: Vec<Spx<'a>>
}

impl<'a> Qs<'a> {
    pub fn new() -> Self {
        Self {
            vertex_q: Vec::with_capacity(50),
            simplex_q: Vec::with_capacity(50)
        }
    }
}

type VertexQs<'a> = Vec<Qs<'a>>;

// *************************************************************
//   PARALLEL HASH TABLE TO STORE WORK QUEUE OF SKINNY TRIANGLES
// *************************************************************
struct HashTriangles<'a> { phantom: PhantomData<&'a bool> }

impl<'a> HashHelper for HashTriangles<'a> {
    type KT = &'a Tri<'a>;
    type ET = &'a Tri<'a>;

    fn empty    () -> Self::ET { &NULL_TRI }
    fn get_key  (v: Self::ET) -> Self::KT { v }
    fn replace_q(_s: Self::ET, _s2: Self::ET) -> bool { false }
    fn hash     (s: Self::KT) -> usize { hash64(s.id as u64) as usize }

    fn cmp (s: Self::KT, s2:Self::KT) -> std::cmp::Ordering {
        s.id.cmp(&s2.id)
    }

    #[inline(always)]
    fn is_empty (s: &Self::ET) -> bool {
        (*s) as *const Tri == &NULL_TRI as *const Tri
    }

    fn cas (p: &mut Self::ET, o: Self::ET, n: Self::ET) -> bool {
        atomic_cas(p, o, n)
    }
}

type TriangleTable<'a> = HashTable<HashTriangles<'a>>;

// Recursive routine for finding a cavity across an edge with
// respect to a vertex p.
// The simplex has orientation facing the direction it is entered.
//
//         a
//         | \ --> recursive call
//   p --> |T c
// enter   | / --> recursive call
//         b
//
//  If p is in circumcircle of T then
//     add T to simplexQ, c to vertexQ, and recurse
fn find_cavity<'a>(t: Spx<'a>, p: &Vtx, q: &mut Qs<'a>) {
    if t.in_circ(p) {
        q.simplex_q.push(t);
        let t = t.rotate();
        find_cavity(t.across(), p, q);
        q.vertex_q.push(t.first_vertex() as *const Vtx as usize);
        let t = t.rotate();
        find_cavity(t.across(), p, q);
    }
}

// Finds the cavity for v and tries to reserve vertices on the
// boundary (v must be inside of the simplex t)
// The boundary vertices are pushed onto q->vertexQ and
// simplices to be deleted on q->simplexQ (both initially empty)
// It makes no side effects to the mesh other than to X->reserve
fn reserve_for_insert<'a>(v: &Vtx, mut t: Spx<'a>, q: &mut Qs<'a>) {
    // each iteration searches out from one edge of the triangle
    for _ in 0..3 {
        q.vertex_q.push(t.first_vertex() as *const Vtx as usize);
        find_cavity(t.across(), v, q);
        t = t.rotate();
    }
    // the maximum id new vertex that tries to reserve a boundary vertex
    // will have its id written.  reserve starts out as -1
    for i in 0..q.vertex_q.len() {
        write_max_i32(
            unsafe { &mut (*(q.vertex_q[i] as usize as *mut Vtx)).reserve },
            v.id
        );
    }
}

// *************************************************************
//   DEALING WITH THE CAVITY
// *************************************************************
#[inline(always)]
fn skinny_tri(t: &Tri) -> bool {
    let min_angle = 30.0;
    if min_angle_check(
        t.vtx[0].unwrap().pt,
        t.vtx[1].unwrap().pt,
        t.vtx[2].unwrap().pt,
        min_angle
    ) { true }
    else { false }
}

#[inline(always)]
fn obtuse(t: &Spx) -> bool {
    let o = t.o as usize;
    let tt = t.t.unwrap();
    let p0 = tt.vtx[(o + 1) % 3].unwrap().pt;
    let v1 = tt.vtx[o].unwrap().pt - p0;
    let v2 = tt.vtx[(o + 2) % 3].unwrap().pt - p0;
    v1.dot(v2) < 0.0
}

#[inline(always)]
fn circumcenter(t: &Spx) -> P {
    let tt = t.t.unwrap();
    if t.is_triangle() {
        triangle_circumcenter(
            tt.vtx[0].unwrap().pt,
            tt.vtx[1].unwrap().pt,
            tt.vtx[2].unwrap().pt
        )
    }
    else { // t.isBoundary()
        let o = t.o as usize;
        let p0 = tt.vtx[(o + 2) % 3].unwrap().pt;
        let p1 = tt.vtx[o].unwrap().pt;
        p0 + (p1 - p0) / 2.0
    }
}

// this side affects the simplex_t by moving it into the right orientation
// and setting the boundary if the circumcenter encroaches on a boundary
#[inline(always)]
fn check_encroached(t: &mut Spx) -> bool {
    if t.is_boundary() { return false }
    let mut i = 0;
    while i < 3 {
        if t.across().is_boundary() && t.far_angle() > 45.0 {
            break;
        } else { *t = t.rotate(); i+=1; }
    }
    if i < 3 {
        t.boundary = true;
        true
    } else { false }
}

fn find_and_reserve_cavity<'a>(
    v: &mut Vtx<'a>,
    t: &mut Spx<'a>,
    q: &mut Qs<'a>
) -> bool {
    *t = Spx::<'a>::new(v.bad_t.unwrap(), 0);
    if t.t.is_none() { panic!("refine: nothing in badT"); }
    if t.t.unwrap().bad == 0 { return false; }

    // if there is an obtuse angle then move across to opposite triangle, repeat
    if obtuse(t) { *t = t.across(); }
    while t.is_triangle() {
        let mut i = 0;
        while i<2 {
            *t = t.rotate();
            if obtuse(t) {
                *t = t.across();
                break;
            }
            i+=1;
        }
        if i==2 { break; }
    }

    // if encroaching on boundary, move to boundary
    check_encroached(t);

    // use circumcenter to add (if it is a boundary then its middle)
    v.pt = circumcenter(t);
    reserve_for_insert(v, *t, q);
    true
}

// checks if v "won" on all adjacent vertices and inserts point if so
// returns true if "won" and cavity was updated
fn add_cavity<'a>(
    v: &mut Vtx,
    t: Spx<'a>,
    q: &mut Qs<'a>,
    tt: &TriangleTable<'a>
) -> bool {
    let mut flag = true;
    for i in 0..q.vertex_q.len() {
        let u = unsafe { &mut *(q.vertex_q[i] as *mut Vtx) };
        if u.reserve == v.id { u.reserve = -1; } // reset to -1
        else { flag = false; } // someone else with higher priority reserved u
    }
    if flag {
        let t0 = t.t.unwrap();
        let t1 = unsafe { make_mut!(v.t.unwrap(), Tri).unwrap() };
        let t2 = unsafe { &mut *(t1 as *mut Tri).add(1) };
        t1.initialized = true;
        if t.is_boundary() { t.split_boundary(v, t1); }
        else {
            t2.initialized = true;
            t.split(v, t1, t2);
        }

        // update the cavity
        for i in 0..q.simplex_q.len() { q.simplex_q[i].flip(); }
        q.simplex_q.push( Spx::new(t0, 0) );
        q.simplex_q.push( Spx::new(t1, 0) );
        if !t.is_boundary() { q.simplex_q.push(Spx::new(t2, 0)); }

        for i in 0..q.simplex_q.len() {
            let t = q.simplex_q[i].t.unwrap();
            if skinny_tri(t) {
                tt.insert(t);
                unsafe { make_mut!(t, Tri).unwrap().bad = 1; }
            }
            else { unsafe { make_mut!(t, Tri).unwrap().bad = 0; } }
        }
        v.bad_t = None;
    }
    q.simplex_q.clear();
    q.vertex_q.clear();
    return flag;
}

// *************************************************************
//    MAIN REFINEMENT LOOP
// *************************************************************

// Insert a set of vertices to refine the mesh
// TT is an initially empty table used to store all the bad
// triangles that are created when inserting vertices
fn add_refining_vertices<'a>(
    vs: &mut [&mut Vtx<'a>],
    tt: &TriangleTable<'a>,
    vq: &mut VertexQs<'a>
) -> usize {
    let n = vs.len();
    let size = n.min(vq.len());

    let mut t: Vec<_> = (0..size)
        .into_par_iter()
        .map(|_| Spx::default())
        .collect();
    let mut flags: Vec<_> = (0..size)
        .into_par_iter()
        .map(|_| false)
        .collect();

    let mut top = n;
    let mut num_failed = 0;

    // process all vertices starting just below the top
    while top > 0 {
        let cnt = size.min(top);
        let offset = top - cnt;

        (
            &mut flags[..cnt],
            &mut vs[offset..top],
            &mut t[..cnt],
            &mut vq[..cnt]
        )
            .into_par_iter()
            .for_each(|(fj, vj, tj, vqj)| {
                *fj = find_and_reserve_cavity(vj, tj, vqj);
            });

        (
            &mut flags[..cnt],
            &mut vs[offset..top],
            &mut t[..cnt],
            &mut vq[..cnt]
        )
            .into_par_iter()
            .for_each(|(fj, vj, tj, vqj)| {
                *fj = *fj && !add_cavity(vj, *tj, vqj, tt);
            });

        // Pack the failed vertices back onto Q
        let mut remain = vec![];
        unsafe { nc_pack(&vs[offset..top], &flags[..cnt], &mut remain); }
        vs[offset .. offset + remain.len()]
            .par_iter_mut()
            .enumerate()
            .for_each(|(j, vj)| unsafe {
                *vj = *(remain.as_ptr() as usize as *mut &mut Vtx).add(j);
            });
        num_failed += remain.len();
        top = top - cnt + remain.len();
    }
    return num_failed;
}

// *************************************************************
//    DRIVER
// *************************************************************

static QSIZE: usize = 20000;

fn refine_internal(tris: &Triangles<P>, dest: &mut Triangles<P>) {
    let mut t = Timer::new("dr"); t.start();
    let expand_factor = 4;
    let n = tris.num_points();
    let m = tris.num_triangles();
    let extra_vertices = expand_factor*n;
    let total_vertices = n + extra_vertices;
    let total_triangles = m + 2 * extra_vertices;

    let (mut triangles, mut vertices) =
        topology_from_triangles(tris, extra_vertices);
    t.next("from Triangles");

    //  set up extra triangles
    triangles[m..total_triangles]
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, ti)| {
            ti.id = i+m;
            ti.initialized = false;
        });

    //  set up extra vertices
    let mut vs = Vec::<&mut Vtx>::with_capacity(extra_vertices);
    unsafe { vs.set_len(extra_vertices); }
    (&mut vs, &mut vertices[n..n + extra_vertices])
        .into_par_iter()
        .enumerate()
        .for_each(|(i, (vsi, vi))| {
            *vi = Vtx::new(Point2d::default(), i + n);
            vi.t = Some(&triangles[m + 2 * i]);
            unsafe { *vsi = (vi as *mut Vtx).as_mut().unwrap() };
        });
    t.next("initializing");

    // these will increase as more are added
    let mut num_points = n;
    let mut num_triangs = m;

    let mut work_q = TriangleTable::new(num_triangs, 1.5);
    triangles[..num_triangs].par_iter().for_each(|ti| {
        if skinny_tri(ti) {
            work_q.insert(ti);
            unsafe { *make_mut!(&ti.bad, u8).unwrap() = 1; }
        }
    });

    let mut vq: Vec<_> = (0..QSIZE)
        .into_par_iter()
        .map(|_| Qs::new())
        .collect();
    t.next("Start");

    // Each iteration processes all bad triangles from the workQ while
    // adding new bad triangles to a new queue
    loop {
        let bad_tt = work_q.entries();

        // packs out triangles that are no longer bad
        let flags: Vec<_> = bad_tt.par_iter().map(|tt| tt.bad != 0).collect();
        let mut bad_t = vec![];
        pack(&bad_tt, &flags, &mut bad_t);
        let num_bad = bad_t.len();

        println!("numBad = {num_bad}  out of {}", bad_tt.len());
        if num_bad == 0 { break; }
        if num_points + num_bad > total_vertices {
            panic!("ran out of vertices");
        }
        let offset = num_points - n;

        // allocate 1 vertex per bad triangle and assign triangle to it
        (
            &mut bad_t[..num_bad],
            &mut vertices[n+offset..n+offset+num_bad]
        )
            .into_par_iter()
            .for_each(|(bti, vi)| {
                unsafe { *make_mut!(&bti.bad, u8).unwrap() = 2; }
                vi.bad_t = Some(bti);
            });

        // the new empty work queue
        work_q = TriangleTable::new(num_bad, 1.5);

        // This does all the work adding new vertices, and any new bad triangles to the workQ
        add_refining_vertices(
            &mut vs[offset..offset+num_bad],
            &work_q,
            &mut vq
        );

        // push any bad triangles that were left untouched onto the Q
        (0..num_bad)
            .into_par_iter()
            .for_each(|i| {
                if bad_t[i].bad==2 {
                    work_q.insert(bad_t[i]);
                }
            });

        num_points += num_bad;
        num_triangs += 2 * num_bad;
    }
    t.next("refinement");
    println!("{num_triangs} : {} : {num_points}", vertices.len());

    // Extract Vertices for result
    let flag: Vec<_> = vertices[..num_points]
        .par_iter()
        .map(|vi| vi.bad_t.is_none())
        .collect();

    let mut is: Vec<usize> = vec![];
    pack_index(&flag, &mut is);
    let n0 = is.len();
    let rp: Vec<_> = (0..n0).into_par_iter().map(|i| {
        unsafe { *make_mut!(&vertices[is[i]].id, i32).unwrap() = i as i32; }
        vertices[is[i]].pt
    }).collect();
    println!("total points = {}", n0);

    // Extract Triangles for result
    let mut is: Vec<usize> = vec![];
    let flags: Vec<_> = triangles[0..num_triangs]
        .par_iter()
        .map(|ti| ti.initialized)
        .collect();
    pack_index(&flags, &mut is);

    let rt: Vec<_> = (0..is.len()).into_par_iter().map(|i| {
        let t = triangles[is[i]];
        [t.vtx[0].unwrap().id, t.vtx[1].unwrap().id, t.vtx[2].unwrap().id]
    }).collect();

    println!("total triangles = {}", is.len());
    t.next("finish");
    *dest = Triangles::new(rp, rt);
}

pub fn refine(tris: &Triangles<P>, dest: &mut Triangles<P>) {
    #[cfg(feature = "AW_safe")]
    eprintln!("Incremental delaunay refinement cannot satisfy AW_safe");

    refine_internal(tris, dest);
}
