use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::sync_channel,
    },
    thread,
    time::Instant,
};

use rand::{thread_rng, Rng};

use clap::{ArgEnum, Clap};

use csv::WriterBuilder;
use rayon::prelude::*;

type VertexId = usize;

#[derive(ArgEnum)]
enum Delimiter {
    WhiteSpace,
    Tab,
    Comma,
}

#[derive(Clap)]
#[clap()]
struct Opts {
    csv_edge_path: String,
    #[clap(short, long, about = "output file")]
    output: Option<String>,
    #[clap(arg_enum, short, long, about = "csv delimiter")]
    delimiter: Delimiter,
    #[clap(short, long, default_value = "20", about = "iteration limit")]
    limit: i64,
}

fn main() {
    println!("LPA!");
    let opts = Opts::parse();

    let now = Instant::now();

    let delimiter = match opts.delimiter {
        Delimiter::WhiteSpace => ' ',
        Delimiter::Tab => '\t',
        Delimiter::Comma => ',',
    };

    let (mut vertices, edges, nedges) = load(opts.csv_edge_path, delimiter);

    println!("vertices: {:?}, edges: {:?}", vertices.len(), nedges);

    let (best_community, best_modularity) = lpa(&mut vertices, &edges, nedges, opts.limit);

    println!(
        "best_community: {}, best_modularity: {}",
        best_community, best_modularity
    );

    if let Some(file_path) = opts.output {
        store(&vertices, file_path, delimiter);
    }

    println!(
        "total time: {:?}s",
        now.elapsed().as_secs() as f64 + now.elapsed().subsec_millis() as f64 * 1e-3
    );
}

fn load(file_path: String, delimiter: char) -> (Vec<VertexId>, Vec<Vec<VertexId>>, usize) {
	let now = Instant::now();

    let (sender, receiver) = sync_channel(1024);

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter as _)
        .comment(Some(b'#'))
        .from_path(file_path)
        .unwrap();
    let (nvertices, nedges) = rdr.headers().unwrap().clone().deserialize(None).unwrap();

    let vertices = (0..nvertices).collect::<Vec<_>>();
    let mut edges: Vec<Vec<VertexId>> = vec![vec![]; nvertices as _];

    thread::spawn(move || {
        let mut records = rdr.deserialize();

        while let Some(Ok((src, dst))) = records.next() {
            sender.send((src, dst)).unwrap();
        }
    });

    let mut iter = receiver.iter();
    while let Some((src, dst)) = iter.next() {
        edges[src as usize].push(dst);
        edges[dst as usize].push(src);
    }

	println!("load time: {:?}s", now.elapsed().as_secs() as f64 + now.elapsed().subsec_millis() as f64 * 1e-3);

    (vertices, edges, nedges)
}

fn store(vertices: &[VertexId], file_path: String, delimiter: char) {
	let now = Instant::now();

    let mut wtr = WriterBuilder::new()
        .delimiter(delimiter as _)
        .from_path(file_path)
        .unwrap();
    for (id, label) in vertices.iter().enumerate() {
        wtr.write_record(&[id.to_string(), label.to_string()])
            .unwrap();
    }

	println!("store time: {:?}s", now.elapsed().as_secs() as f64 + now.elapsed().subsec_millis() as f64 * 1e-3);
}

fn lpa(
    vertices: &mut Vec<VertexId>,
    edges: &Vec<Vec<VertexId>>,
    nedges: usize,
    limit: i64,
) -> (usize, f64) {
    // naive random select function
    // let rand = || Instant::now().elapsed().as_nanos() & 1 == 1;

    let (community, modularity) = statistics(vertices, &edges, nedges);
    println!(
        "INIT | community: {:?} modularity: {:?}",
        community, modularity
    );

    let atomic_vertices = vertices
        .par_iter()
        .map(|&x| AtomicU64::new(x as _))
        .collect::<Vec<_>>();

    let active = AtomicU64::new(vertices.len() as _);

    let mut iteration = 0;

    let (mut best_community, mut best_modularity) = (0, -1.0);

    while iteration < limit && active.load(Ordering::Relaxed) > 0 {
        let now = Instant::now();
        active.store(0, Ordering::Relaxed);
        atomic_vertices
            .par_iter()
            .enumerate()
            .for_each(|(id, label)| {
                let mut rng = thread_rng();
                let mut counter = 0;

                let (mut new_label, mut max_count) = (label.load(Ordering::Relaxed), 0);
                let mut label_counts: HashMap<VertexId, VertexId> = HashMap::new();
                for &nbr in edges[id].iter() {
                    let nbr_label = atomic_vertices[nbr as usize].load(Ordering::Acquire);
                    let count = if let Some(count) = label_counts.get_mut(&(nbr_label as _)) {
                        *count += 1;
                        *count
                    } else {
                        label_counts.insert(nbr_label as _, 1);
                        1
                    };
                    if count > max_count {
                        max_count = count;
                        new_label = nbr_label;
                        counter = 1;
                    } else if count == max_count {
                        // reservoir sampling
                        if rng.gen_ratio(1, counter + 1) {
                            new_label = nbr_label;
                        }
                        counter += 1;
                    }
                    // if count > max_count || (count == max_count && rand()) {
                    //     max_count = count;
                    //     new_label = nbr_label;
                    // }
                }

                if label.swap(new_label, Ordering::Release) != new_label {
                    active.fetch_add(1, Ordering::Relaxed);
                }
            });

        let new_vertices = atomic_vertices
            .par_iter()
            .map(|x| x.load(Ordering::Relaxed) as VertexId)
            .collect();

        let (community, modularity) = statistics(&new_vertices, &edges, nedges);

        if modularity > best_modularity {
            *vertices = new_vertices;
            best_community = community;
            best_modularity = modularity;
        }

        println!(
            "iteration {:?} | active: {:?} community: {:?} modularity: {:?} time: {:?}s",
            iteration,
            active,
            community,
            modularity,
            now.elapsed().as_secs() as f64 + now.elapsed().subsec_millis() as f64 * 1e-3
        );
        iteration += 1;
    }

    (best_community, best_modularity)
}

fn statistics(vertices: &Vec<VertexId>, edges: &Vec<Vec<VertexId>>, nedges: usize) -> (usize, f64) {
    let mut communities = vec![HashSet::new(); vertices.len()];
    let mut communities_count = HashSet::new();

    vertices.iter().enumerate().for_each(|(id, &label)| {
        communities[label].insert(id);
        communities_count.insert(label);
    });

    let modularity = communities
        .par_iter()
        .map(|community| {
            if community.len() == 0 {
                return 0.0;
            }
            let mut lv = 0;
            let mut dv = 0;
            for &id in community {
                for nbr in edges[id].iter() {
                    if community.contains(nbr) {
                        lv += 1;
                    }
                }
                dv += edges[id].len();
            }
            let m2 = (nedges * 2) as f64;
            lv as f64 / m2 - (dv as f64 / m2) * (dv as f64 / m2)
        })
        .sum::<f64>();

    (communities_count.len(), modularity)
}
