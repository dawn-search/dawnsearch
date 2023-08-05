use rand::prelude::*;

use dawnsearch::vector::{distance_i8, EM_LEN};

type Embedding = [i8; EM_LEN];

struct Node {
    id: usize,
    address: Embedding,
    route_table: Vec<Vec<usize>>,
}

const NODES: usize = 1000;
const INITIAL_PEERS: usize = 10;
const NODES_PER_BUCKET: usize = 20;
const BUCKETS: usize = 1000;

fn random_address() -> Embedding {
    let mut rng = rand::thread_rng();
    let mut address: Embedding = [0; EM_LEN];
    for x in 0..EM_LEN {
        address[x] = rng.gen();
    }
    address
}

fn closest_node(node: &Node, target: &Embedding, nodes: &Vec<Node>) -> (usize, u32) {
    let mut closest = None;
    let mut closest_distance = 0;
    for bucket in &node.route_table {
        for other_id in bucket {
            let other = &nodes[*other_id as usize];
            if node.id == other.id {
                continue;
            }
            let distance = distance_i8(&other.address, target);
            if closest.is_none() || distance < closest_distance {
                closest = Some(other.id);
                closest_distance = distance;
                continue;
            }
        }
    }
    (closest.unwrap(), closest_distance)
}

fn closest_node_overall<'a>(target: &Embedding, nodes: &'a Vec<Node>) -> (usize, u32) {
    let mut closest = None;
    let mut closest_distance = 0;
    for other in nodes {
        let distance = distance_i8(&other.address, target);
        if closest.is_none() || distance < closest_distance {
            closest = Some(other.id);
            closest_distance = distance;
            continue;
        }
    }
    (closest.unwrap(), closest_distance)
}

fn update_routing(nodes: &mut Vec<Node>, node_id: usize, other_id: usize) {
    let max_distance2: f32 = EM_LEN as f32 * 256.0 * 256.0;
    let distance2 = distance_i8(&nodes[node_id].address, &nodes[other_id].address) as usize;
    let bucket_index = (distance2 as f32).sqrt() / max_distance2.sqrt() * BUCKETS as f32;

    let bucket = &mut nodes[node_id].route_table[bucket_index as usize];

    if bucket.len() >= NODES_PER_BUCKET {
        return;
        // bucket.remove(0);
    }
    if !bucket.contains(&other_id) {
        bucket.push(other_id);
    }
}

fn main() -> anyhow::Result<()> {
    // Generate nodes with random ID.
    let mut rng = rand::thread_rng();

    let mut nodes = Vec::new();
    for id in 0..NODES {
        nodes.push(Node {
            id,
            address: random_address(),
            route_table: Vec::new(),
        });
    }

    // Fill route tables.
    for node_id in 0..NODES {
        for _r in 0..BUCKETS {
            nodes[node_id].route_table.push(Vec::new());
        }
        for _r in 0..INITIAL_PEERS {
            let other_id = rng.gen_range(0..NODES);
            if other_id == node_id {
                continue;
            }
            update_routing(&mut nodes, node_id, other_id);
        }
    }

    let mut count = 0;
    let mut success = 0;
    let debug = false;
    loop {
        count += 1;
        if count >= 10000 {
            let ratio = success as f32 / count as f32;
            println!("Success ratio {}", ratio);
            count = 0;
            success = 0;
        }

        // Perform queries on random data to see if we reach the closest peer.
        let origin_id = rng.gen_range(0..NODES);
        let mut current_id = origin_id;
        let target = random_address();
        let mut best_distance = distance_i8(&nodes[origin_id].address, &target);
        if debug {
            println!("Starting at node {}", origin_id);
        }

        for (b, bucket) in nodes[origin_id].route_table.iter().enumerate() {
            for peer in bucket {
                let peer = &nodes[*peer as usize];
                let dist = distance_i8(&nodes[origin_id].address, &peer.address);
                if debug {
                    println!("Bucket {} peer {} distance {}", b, peer.id, dist);
                }
            }
        }

        loop {
            let (closest_id, closest_distance) = closest_node(&nodes[current_id], &target, &nodes);
            if debug {
                println!(
                    "Querying from {}, closest node is {} distance {}",
                    current_id, closest_id, closest_distance
                );
            }
            update_routing(&mut nodes, origin_id, closest_id);
            update_routing(&mut nodes, closest_id, origin_id);
            if closest_distance >= best_distance {
                break;
            }
            best_distance = closest_distance;
            current_id = closest_id;
        }
        if debug {
            println!("");
            println!(
                "Final closest node is {} distance {}",
                current_id, best_distance
            );
        }

        // Find the real closest node.
        let (real_closest, real_closest_distance) = closest_node_overall(&target, &nodes);
        if real_closest == current_id {
            if debug {
                println!("Success");
            }
            success += 1;
        } else {
            if debug {
                println!(
                    "Fail, real closest node is {} distance {}",
                    real_closest, real_closest_distance
                );
            }
        }
    }

    // Ok(())
}
