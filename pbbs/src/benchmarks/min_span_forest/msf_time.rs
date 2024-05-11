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

#[path ="mod.rs"] mod msf;
#[path ="../../misc.rs"] mod misc;
#[path ="../macros.rs"] mod macros;
#[path ="../../common/io.rs"] mod io;
#[path ="../../common/graph.rs"] mod graph;
#[path ="../../common/graph_io.rs"] mod graph_io;
#[path ="../../algorithm/union_find.rs"] mod union_find;

use misc::*;
use graph::WghEdgeArray;
use io::write_slice_to_file_seq;
use graph_io::read_wgh_edge_array_from_file;
use msf::{ incremental_msf, serial_msf, inc_msf_mod };

define_args!(Algs::INCREMENTAL);

define_algs!(
    (SERIAL, "serial"),
    (INCREMENTAL, "incremental"),
    (INCMOD, "incmod")
);


pub fn run(
    alg: Algs,
    rounds: usize,
    ea: WghEdgeArray
) -> (Vec<DefInt>, Duration)
{
    let sf = match alg {
        Algs::SERIAL => { serial_msf::minimum_spanning_forest },
        Algs::INCREMENTAL => { incremental_msf::minimum_spanning_forest },
        Algs::INCMOD => { inc_msf_mod::minimum_spanning_forest },
    };

    let mut r = vec![];
    let mut ea_copy = ea.clone();
    let ea_copy_shadow = unsafe {
        (&ea_copy as *const WghEdgeArray).as_ref().unwrap()
    };
    let mean = time_loop(
        "msf",
        rounds,
        Duration::new(1, 0),
        || { if alg == Algs::INCMOD { ea_copy = ea.clone(); }},
        || { sf(&ea_copy_shadow, &mut r); },
        || {}
    );
    (r, mean)
}

fn main() {
    init!();
    let args = Args::parse();
    let ea = read_wgh_edge_array_from_file(&args.ifname);
    let (r, d) = run(args.algorithm, args.rounds, ea);

    finalize!(
        args,
        r,
        d,
        write_slice_to_file_seq(&r, args.ofname)
    );
}
