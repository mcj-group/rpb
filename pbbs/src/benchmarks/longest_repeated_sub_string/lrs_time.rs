
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

#[path ="mod.rs"] mod lrs;
#[path ="../../misc.rs"] mod misc;
#[path ="../macros.rs"] mod macros;
#[path ="../../common/io.rs"] mod io;
#[path ="../../algorithm/lcp.rs"] mod lcp;
#[path ="../../algorithm/range_min.rs"] mod range_min;
#[path ="../../algorithm/suffix_array.rs"] mod suffix_array;

use misc::*;
use lrs::doubling;
use io::{chars_from_file, chars_to_file};

define_args!(Algs::Doubling);
define_algs!((Doubling, "doubling"));


pub fn run(
    alg: Algs,
    rounds: usize,
    inp: &[DefChar]
) -> ((usize, usize, usize), Duration)
{
    let f = match alg {
        Algs::Doubling => {doubling::lrs},
    };

    let mut r = (0, 0, 0);

    let mean = time_loop(
        "lrs",
        rounds,
        Duration::new(1, 0),
        || {},
        || { r = f(&inp); },
        || {}
    );
    (r, mean)
}

fn main() {
    init!();
    let args = Args::parse();
    let arr = chars_from_file(&args.ifname, false).unwrap();
    let ((len, loc1, loc2), d) = run(args.algorithm, args.rounds, &arr);

    let out = format!("len:{len}\tloc1:{loc1}\tloc2:{loc2}");
    if !args.ofname.is_empty() {
        chars_to_file(out.as_bytes(), args.ofname).unwrap();
    } else { println!("{}", out); }

    println!("mean_seconds: {:.9}", d.as_secs_f64());
}
