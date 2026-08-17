use std::io::Write;
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

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;
use clap::Parser;
use rayon::prelude::*;

use multiqueue::MultiQueue;
use multiqueue::util::WghGraph as Graph;
use multiqueue::util::termination_detection::{TerminationData, try_do};


#[derive(Eq, PartialEq, Debug, Clone, Copy)]
struct ValType(usize, usize);

impl Ord for ValType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}

impl PartialOrd for ValType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.0.cmp(&self.0))
    }
}


struct SharedData {
    shortest_distance: Vec<AtomicUsize>,
    term_data: TerminationData,
}

fn process_node(
    val: ValType,
    graph: &Graph,
    data: &SharedData,
    pq: &MultiQueue<ValType>
) {
    let current_distance = data.shortest_distance[val.1].load(Ordering::Relaxed);
    if val.0 > current_distance {
        // TODO: data.ignored_nodes+=1;
        return;
    }
    // TODO: data.processed_nodes+=1;
    for i in graph.nodes[val.1]..graph.nodes[val.1 + 1] {
        let target = graph.edges[i].target;
        let new_distance = val.0 + graph.edges[i].weight;
        let mut old_distance = data.shortest_distance[target].load(Ordering::Relaxed);

        while new_distance < old_distance {
            match data.shortest_distance[target].compare_exchange_weak(
                old_distance,
                new_distance,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    pq.push(ValType(new_distance, target));
                    // data.pushed_nodes+=1;
                    break;
                },
                Err(x) => old_distance = x,
            }
        }
    }
}

fn main_loop(
    graph: &Graph,
    pq: &MultiQueue<ValType>,
    data: &SharedData,
) {
    while let Ok(val) = try_do(
        &|| if let Some(val) = pq.pop() { Ok(val) } else { Err(()) },
        &data.term_data,
    ) {
        process_node(val, graph, data, pq);
    }
}

fn launch_threads_and_wait(
    graph: &Graph,
    num_threads: usize,
    pq: &MultiQueue<ValType>,
    data: &SharedData,
    start_node: usize,
) {
    pq.push(ValType(0, start_node));
    data.shortest_distance[start_node].store(0, Ordering::Relaxed);

    thread::scope(|s| {
        for _ in 0..num_threads {
            s.spawn(|| main_loop(graph, pq, data));
        }
    });
}

// Verification method adopted from Galois
fn verify_distance(
    graph: &Graph,
    distance: &[AtomicUsize]
) {
    let mut failed = false;
    let max = AtomicUsize::new(0);
    distance.iter().enumerate().for_each(|(v,x)| {
        let dist = x.load(Ordering::Relaxed);
        if dist != usize::MAX {
            for i in graph.nodes[v]..graph.nodes[v + 1] {
                let target = graph.edges[i].target;
                let target_dist = distance[target].load(Ordering::Relaxed);
                let edge_weight = graph.edges[i].weight;
                if target_dist > dist + edge_weight {
                    failed = true;
                }
            }
            let mut cur_max = max.load(Ordering::Relaxed);
            while dist > cur_max {
                match max.compare_exchange_weak(
                    cur_max,
                    dist,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => { break; },
                    Err(new_max) => cur_max = new_max,
                }
            }
        }
    });

    if failed {
        println!("Verification Failed");
    } else {
        println!("Verification Success");
    }
    println!("max distance: {:.6}", max.load(Ordering::Relaxed));
}

fn write_distance<P: AsRef<Path>>(path: P, distance: &[AtomicUsize]) {
    let mut file = std::fs::File::create(path).unwrap();
    distance.iter().for_each(|x| {
        writeln!(file, "{}", x.load(Ordering::Relaxed)).unwrap();
    });
}

#[derive(Parser)]
struct Args {
    file: String,
    #[arg(short, long)]
    output: Option<String>,
    #[arg(short, long, default_value = "1")]
    rounds: usize,
    #[arg(short, long)]
    solution: Option<String>,
    #[arg(short, long, required = true)]
    threads: usize,
    #[arg(long)]
    start_node: Option<usize>,
    #[arg(short, long)]
    verify: bool,
}

fn main() {
    let args = Args::parse();
    let graph = Graph::from_file(args.file);

    let mut data = SharedData {
        shortest_distance: (0..graph.num_nodes()).into_par_iter()
            .map(|_| AtomicUsize::new(usize::MAX))
            .collect(),
        term_data: TerminationData::new(args.threads),
    };
    let mut times = vec![];
    for _ in 0..args.rounds {
        // initialization
        let pq = MultiQueue::new(args.threads);
        data.shortest_distance = (0..graph.num_nodes()).into_par_iter()
            .map(|_| AtomicUsize::new(usize::MAX))
            .collect();
        data.term_data = TerminationData::new(args.threads);

        // run
        let start = Instant::now();
        launch_threads_and_wait(
            &graph,
            args.threads,
            &pq,
            &data,
            args.start_node.unwrap_or(0),
        );
        let e = start.elapsed();
        println!("sssp:\t{:.6}", e.as_secs_f64());
        times.push(e);
    }

    let mean = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("mean_seconds: {:.9}", mean.as_secs_f64());
    if let Some(output) = args.output {
        write_distance(output, &data.shortest_distance);
    }

    if args.verify {
        verify_distance(&graph, &data.shortest_distance);
    }
}
