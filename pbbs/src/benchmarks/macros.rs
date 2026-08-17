#[macro_export]
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

macro_rules! define_algs {
    ($(($alg: ident, $name: expr)),*) => {
        use std::fmt;
        use clap::ValueEnum;
        #[path ="../../common/time_loop.rs"] mod time_loop;
        use time_loop::time_loop;

        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
        pub enum Algs { $($alg,)* }

        impl fmt::Display for Algs {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(Algs::$alg => write!(f, $name),)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! define_args {
    ($default_alg: expr $(, ($name: ident, $type: ty, $default: expr))*) => {
        use clap::Parser;

        #[derive(Parser, Debug)]
        #[clap(version, about, long_about = None)]
        struct Args {
            /// the algorithm to use
            #[clap(short, long, value_parser, default_value_t = $default_alg)]
            algorithm: Algs,

            /// the output filename
            #[clap(short, long, required=false, default_value_t = ("").to_string())]
            ofname: String,

            /// the input filename
            #[clap(value_parser, required=true)]
            ifname: String,

            /// the number of rounds to execute the benchmark
            #[clap(short, long, value_parser, required=false, default_value_t=1)]
            rounds: usize,

            $(#[clap(long, value_parser, required=false, default_value_t=$default)]
            $name: $type,)*
        }
    }
}

#[macro_export]
macro_rules! init {
    () => {
        use rayon::prelude::*;
        use affinity::set_thread_affinity;

        // pin rayon's threads to cores
        // TODO: find a better way to do this.
        (0..rayon::current_num_threads())
        .par_bridge()
        .for_each(|_| {
            set_thread_affinity(
                [rayon::current_thread_index().unwrap()]
            ).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100))
        })
    }
}

#[macro_export]
macro_rules! finalize {
    ($args: ident, $r: ident, $d: ident, $write: expr) => {
        if !$args.ofname.is_empty() {
            $write
        } else {
            if $r.len() < 20 { println!("result:  {:?}", $r); }
            else { println!("result:  {:?} ... [Ommited]", &$r[..20]); }
        }

        println!("mean_seconds: {:.9}", $d.as_secs_f64());
    }
}
