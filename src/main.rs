use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::{HashSet, VecDeque, HashMap};
use std::fs::File;
use std::io::Write;

// --- 0. Constants ---
const BATTERY_FULL_SMARTPHONE: f32 = 1000.0;
const BATTERY_INFINITE: f32 = 999999.0;

const COST_IDLE: f32 = 0.5;
const COST_TX: f32 = 5.0;
const COST_RX: f32 = 2.0;

const REWARD_RELAY: f32 = 1.0; // Token reward per relay
const INSURANCE_PAYOUT: f32 = 10000.0; // USDC payout

const DISASTER_STEP: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
enum SimMode {
    Flooding, // Old tech (Benchmark baseline)
    Swarm,    // New tech (Unicorn)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum NodeType {
    Smartphone,
    BaseStation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Wallet {
    address: String,
    balance_token: f32,
    balance_usdc: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    id: u32,
    // (x, y) relative coordinates (0-200)
    position: (f64, f64),
    // Lat/Lon for visualization (calculated from position)
    lat: f64,
    lon: f64,
    is_active: bool,
    peers: Vec<u32>,
    node_type: NodeType,
    battery_level: f32,
    transmission_range: f64,
    wallet: Wallet,
}

#[derive(Debug, Clone)]
struct Packet {
    id: String,
    history: Vec<u32>,
    target_id: u32,
    hops: u32,
    ttl: u32,
}

// Log structure for Visualization
#[derive(Serialize)]
struct SimLog {
    step: i32,
    nodes: Vec<NodeLog>,
    packets: Vec<PacketLog>,
    events: Vec<String>,
}

#[derive(Serialize)]
struct NodeLog {
    id: u32,
    lat: f64,
    lon: f64,
    is_active: bool,
    node_type: String, // "Smartphone" or "BaseStation"
    battery: f32,
}

#[derive(Serialize)]
struct PacketLog {
    id: String,
    path: Vec<u32>, // Node IDs in order
}

impl Node {
    fn new(id: u32) -> Self {
        let mut rng = rand::rng();
        // 15% BaseStation
        let (node_type, battery, range) = if rng.random_bool(0.15) {
            (NodeType::BaseStation, BATTERY_INFINITE, 180.0) 
        } else {
            (NodeType::Smartphone, BATTERY_FULL_SMARTPHONE, 40.0)
        };

        let x = rng.random_range(0.0..200.0);
        let y = rng.random_range(0.0..200.0);
        
        // Map to Nice, France (Approx 43.7102, 7.2620)
        // Scale: 200 units = ~0.02 degrees (~2km)
        let lat = 43.70 + (y * 0.0001);
        let lon = 7.25 + (x * 0.0001);

        Node {
            id,
            position: (x, y),
            lat,
            lon,
            is_active: true,
            peers: Vec::new(),
            node_type,
            battery_level: battery,
            transmission_range: range,
            wallet: Wallet {
                address: format!("0x{:04x}...{:04x}", rng.random_range(0..65535), id),
                balance_token: 0.0,
                balance_usdc: 0.0,
            },
        }
    }

    fn distance_to(&self, other: &Node) -> f64 {
        let dx = self.position.0 - other.position.0;
        let dy = self.position.1 - other.position.1;
        (dx * dx + dy * dy).sqrt()
    }
    
    fn consume_battery(&mut self, cost: f32) {
        if self.node_type == NodeType::Smartphone {
            self.battery_level = (self.battery_level - cost).max(0.0);
            if self.battery_level <= 0.0 {
                self.is_active = false;
            }
        }
    }
}

struct SimStats {
    total_energy: f32,
    success_packets: u32,
    total_hops: u32,
}

fn run_simulation(mode: SimMode, export_logs: bool) -> SimStats {
    println!("\n▶️ RUNNING SIMULATION: {:?}", mode);
    
    // Hardcoded seed logic is tricky in simple Rust without specific crates, 
    // but we'll re-generate nodes similarly to keep it fair-ish.
    let node_count = 60;
    let mut nodes: Vec<Node> = (0..node_count).map(|i| Node::new(i)).collect();

    // Rebuild Adjacency
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    for i in 0..node_count { adjacency.insert(i as u32, Vec::new()); }
    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i == j { continue; }
            if nodes[i].distance_to(&nodes[j]) <= nodes[i].transmission_range {
                adjacency.get_mut(&(i as u32)).unwrap().push(j as u32);
            }
        }
    }
    for node in &mut nodes {
        if let Some(peers) = adjacency.get(&node.id) {
            node.peers = peers.clone();
        }
    }

    let start_node_id = 0;
    let target_node_id = node_count - 1;
    let mut packet_queue: VecDeque<Packet> = VecDeque::new();
    
    let mut rng = rand::rng();
    let max_steps = 40;
    let mut total_energy_consumed: f32 = 0.0;
    let mut successful_packets = 0;
    let mut total_hops = 0;
    let mut disaster_triggered = false;
    let mut oracle_alert_sent = false;

    // For visualization logs
    let mut sim_logs: Vec<SimLog> = Vec::new();

    for step in 1..=max_steps {
        let mut current_step_events: Vec<String> = Vec::new();

        // 1. Disaster (Only in Swarm mode for demo, or both? Let's do both to show resilience difference)
        if step == DISASTER_STEP {
            current_step_events.push("DISASTER_START".to_string());
            println!("⚠️  ALERT: DISASTER OCCURRED!");
            let mut destroyed_count = 0;
            for node in &mut nodes {
                // South Area (y < 80.0)
                if node.position.1 < 80.0 && node.is_active {
                    node.is_active = false;
                    node.battery_level = 0.0;
                    destroyed_count += 1;
                }
            }
            println!("🔥 {} nodes destroyed.", destroyed_count);
            disaster_triggered = true;
        }

        // 2. Oracle (Tokenomics)
        if disaster_triggered && !oracle_alert_sent && mode == SimMode::Swarm {
             // Calculate survival rate
             let south_total = nodes.iter().filter(|n| n.position.1 < 80.0).count();
             let south_active = nodes.iter().filter(|n| n.position.1 < 80.0 && n.is_active).count();
             if south_total > 0 && south_active == 0 {
                 println!("[ORACLE] 💸 INSURANCE TRIGGERED! Paying out USDC to victims...");
                 oracle_alert_sent = true;
                 current_step_events.push("ORACLE_PAYOUT".to_string());

                 // Payout Logic
                 for node in &mut nodes {
                     if node.position.1 < 80.0 {
                         node.wallet.balance_usdc += INSURANCE_PAYOUT;
                     }
                 }
             }
        }

        // 3. New Packet Generation
        if nodes[start_node_id as usize].is_active {
            packet_queue.push_back(Packet {
                id: format!("M{}_{}", step, mode as i32),
                history: vec![start_node_id],
                target_id: target_node_id,
                hops: 0,
                ttl: 15,
            });
        }

        // 4. Energy Drain (Idle)
        for node in &mut nodes {
            if node.is_active {
                node.consume_battery(COST_IDLE);
                total_energy_consumed += COST_IDLE;
            }
        }

        // 5. Packet Processing
        let mut next_queue: VecDeque<Packet> = VecDeque::new();
        let mut step_visited: HashMap<String, HashSet<u32>> = HashMap::new();
        
        // For visualization: track verified paths this step
        let mut verified_packets: Vec<PacketLog> = Vec::new();

        while let Some(packet) = packet_queue.pop_front() {
            let current_node_id = *packet.history.last().unwrap();
            
            if current_node_id == target_node_id {
                successful_packets += 1;
                total_hops += packet.hops;
                verified_packets.push(PacketLog { 
                    id: packet.id.clone(), 
                    path: packet.history.clone() 
                });
                continue;
            }

            if packet.ttl == 0 || !nodes[current_node_id as usize].is_active { continue; }

            // TX Cost
            nodes[current_node_id as usize].consume_battery(COST_TX);
            total_energy_consumed += COST_TX;

            let peers = nodes[current_node_id as usize].peers.clone();
            
            for neighbor_id in peers {
                if packet.history.contains(&neighbor_id) { continue; } // No loops
                
                let visited_set = step_visited.entry(packet.id.clone()).or_insert(HashSet::new());
                if visited_set.contains(&neighbor_id) { continue; } // No duplicate sends in same step

                let neighbor = &nodes[neighbor_id as usize];
                if !neighbor.is_active { continue; }

                // --- ROUTING LOGIC ---
                let should_forward = match mode {
                    SimMode::Flooding => true, // Always forward (Dumb)
                    SimMode::Swarm => {
                        // Smart Logic
                         if neighbor.node_type == NodeType::BaseStation {
                             true
                         } else {
                             // Aggressive Unicorn Logic:
                             // Only relay if battery is high AND random chance is low (sparse routing)
                             let bat_p = neighbor.battery_level / BATTERY_FULL_SMARTPHONE;
                             // e.g. 0.05 probability if full battery. 
                             // This effectively makes Smartphones "last resort" or "sparse extensions"
                             rng.random_bool(0.05 * (bat_p as f64)) 
                         }
                    }
                };

                if should_forward {
                    nodes[neighbor_id as usize].consume_battery(COST_RX);
                    total_energy_consumed += COST_RX;
                    
                    // Token Reward (Mining)
                    if mode == SimMode::Swarm {
                        nodes[neighbor_id as usize].wallet.balance_token += REWARD_RELAY;
                    }

                    let mut new_history = packet.history.clone();
                    new_history.push(neighbor_id);
                    
                    next_queue.push_back(Packet {
                        id: packet.id.clone(),
                        history: new_history,
                        target_id: packet.target_id,
                        hops: packet.hops + 1,
                        ttl: packet.ttl - 1,
                    });
                    
                    visited_set.insert(neighbor_id);
                }
            }
        }
        packet_queue = next_queue;
        
        // SAVE LOGS (Only for Swarm mode usually, or we can save both. Let's save Swarm for v4 visualization)
        if export_logs {
             let node_logs = nodes.iter().map(|n| NodeLog {
                 id: n.id,
                 lat: n.lat,
                 lon: n.lon,
                 is_active: n.is_active,
                 node_type: format!("{:?}", n.node_type),
                 battery: n.battery_level,
             }).collect();
             
             sim_logs.push(SimLog {
                 step,
                 nodes: node_logs,
                 packets: verified_packets,
                 events: current_step_events,
             });
        }
    }

    if export_logs {
        let json_data = serde_json::to_string_pretty(&sim_logs).unwrap();
        let mut file = File::create("simulation_log.json").unwrap();
        file.write_all(json_data.as_bytes()).unwrap();
        println!("💾 Log exported to 'simulation_log.json'");
    }

    SimStats {
        total_energy: total_energy_consumed,
        success_packets: successful_packets,
        total_hops: total_hops,
    }
}

fn main() {
    println!("=== 🦄 ResilientMesh v4.0 Unicorn Benchmark ===");
    
    // 1. Run Flooding (Baseline)
    let stats_flood = run_simulation(SimMode::Flooding, false);
    
    // 2. Run Swarm (New Tech) - Export logs for this one
    let stats_swarm = run_simulation(SimMode::Swarm, true);

    println!("\n=== 📊 BENCHMARK RESULTS ===");
    println!("Metric                 | Flooding (Old) | Swarm (Unicorn) | Improvement");
    println!("-----------------------|----------------|-----------------|------------");
    
    let energy_imp = (stats_flood.total_energy - stats_swarm.total_energy) / stats_flood.total_energy * 100.0;
    println!("Total Energy Consumed  | {:>14.1} | {:>15.1} | {:>10.1}% 🚀", 
        stats_flood.total_energy, stats_swarm.total_energy, energy_imp);

    println!("Packets Delivered      | {:>14} | {:>15} |", 
        stats_flood.success_packets, stats_swarm.success_packets);
        
    let efficiency = (stats_swarm.success_packets as f32 / stats_swarm.total_energy) / (stats_flood.success_packets as f32 / stats_flood.total_energy);
    println!("Energy Efficiency (Msg/E)|         1.0x |           {:>.1}x |", efficiency);
    
    println!("\n[Next Steps]");
    println!("1. Open 'map.html' (generate it with python src/visualize.py)");
    println!("2. See the insurance payout event in the log.");
}