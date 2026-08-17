#![allow(dead_code)]
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


use std::time::Duration;

#[path ="mod.rs"] mod dr;
#[path ="../../misc.rs"] mod misc;
#[path ="../macros.rs"] mod macros;
#[path ="../../common/mod.rs"] mod common;

use misc::*;
use dr::incremental;
use common::geometry::{Triangles, Point2d};
use common::geometry_io::{read_triangles_from_file, write_triangles_to_file};

type P = Point2d<f64>;

define_args!(Algs::INCREMENTAL);

define_algs!((INCREMENTAL, "incremental"));

pub fn run(
    alg: Algs,
    rounds: usize,
    tris: &Triangles<P>
) -> (Triangles<P>, Duration) {
    let f = match alg {
        Algs::INCREMENTAL => incremental::refine,
    };

    let mut r = Triangles { p: vec![], t: vec![] };
    let mean = time_loop(
        "dr",
        rounds,
        Duration::new(1, 0),
        || {},
        || { f(tris, &mut r); },
        || {}
    );
    (r, mean)
}

fn main() {
    init!();
    let args = Args::parse();
    let tris = read_triangles_from_file(&args.ifname, 0);
    let (r, d) = run(args.algorithm, args.rounds, &tris);

    if !args.ofname.is_empty() { write_triangles_to_file(&r, args.ofname); }
    println!("mean_seconds: {:.9}", d.as_secs_f64());
}
