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

use std::collections::BinaryHeap;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use clap::Parser;

use multiqueue::util::WghGraph as Graph;


#[derive(Eq, PartialEq, Debug)]
struct ValType(usize, usize);

impl Ord for ValType {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

impl PartialOrd for ValType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.0.cmp(&self.0))
    }
}

fn dijkstra(
    graph: &Graph,
    pq: &mut BinaryHeap<ValType>,
    dist: &mut Vec<usize>,
) {
    while let Some(val) = pq.pop() {
        for i in graph.nodes[val.1]..graph.nodes[val.1 + 1] {
            let target = graph.edges[i].target;
            let new_distance = val.0 + graph.edges[i].weight;
            if new_distance < dist[target] {
                dist[target] = new_distance;
                pq.push(ValType(new_distance, target));
            }
        }
    }
}

fn write_distance<P: AsRef<Path>>(path: P, distance: &[usize]) {
    let mut file = std::fs::File::create(path).unwrap();
    distance.iter().for_each(|x| writeln!(file, "{}", x).unwrap());
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
    #[arg(long)]
    start_node: Option<usize>,
}

fn main() {
    let args = Args::parse();
    let graph = Graph::from_file(args.file);

    let mut times = vec![];
    let mut dist = vec![usize::MAX; graph.num_nodes()];
    for _ in 0..args.rounds {
        // initialization
        let mut pq = BinaryHeap::new();
        dist.iter_mut().for_each(|x| *x = usize::MAX);

        // first node
        let start_node = args.start_node.unwrap_or(0);
        pq.push(ValType(0, start_node));
        dist[start_node] = 0;

        // run
        let start = Instant::now();
        dijkstra(&graph, &mut pq, &mut dist);
        let e = start.elapsed();
        println!("sssp:\t{:.6}", e.as_secs_f64());
        times.push(e);
    }

    let mean = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    println!("mean_seconds: {:.9}", mean.as_secs_f64());
    if let Some(output) = args.output {
        write_distance(output, &dist);
    }
}
