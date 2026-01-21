use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::{HashSet, VecDeque, HashMap};

// --- 0. 定数定義 ---
const BATTERY_FULL_SMARTPHONE: f32 = 1000.0;
const BATTERY_INFINITE: f32 = 999999.0;

const COST_IDLE: f32 = 0.5;
const COST_TX: f32 = 5.0;
const COST_RX: f32 = 2.0;

// 災害発生タイミング
const DISASTER_STEP: i32 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum NodeType {
    Smartphone,
    BaseStation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    id: u32,
    position: (f64, f64),
    is_active: bool,
    peers: Vec<u32>,
    node_type: NodeType,
    battery_level: f32,
    transmission_range: f64,
}

#[derive(Debug, Clone)]
struct Packet {
    id: String,
    history: Vec<u32>,
    target_id: u32,
    hops: u32,
    ttl: u32, // Time To Live (無限ループ防止)
}

impl Node {
    fn new(id: u32) -> Self {
        let mut rng = rand::rng();
        // 15% BaseStation (少し増やす)
        let (node_type, battery, range) = if rng.random_bool(0.15) {
            (NodeType::BaseStation, BATTERY_INFINITE, 180.0) 
        } else {
            (NodeType::Smartphone, BATTERY_FULL_SMARTPHONE, 40.0)
        };

        Node {
            id,
            position: (rng.random_range(0.0..200.0), rng.random_range(0.0..200.0)),
            is_active: true,
            peers: Vec::new(),
            node_type,
            battery_level: battery,
            transmission_range: range,
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

// --- 群知能ルーティング (Swarm Logic) ---
// 次のホップを選ぶ際の確率計算
// BaseStation -> 優先度高
// Smartphone (High Battery) -> 優先度中
// Smartphone (Low Battery) -> 優先度低
fn should_relay(node: &Node, rng: &mut impl Rng) -> bool {
    match node.node_type {
        NodeType::BaseStation => true, // 常に中継
        NodeType::Smartphone => {
            let battery_percent = node.battery_level / BATTERY_FULL_SMARTPHONE;
            // バッテリー残量が確率になる (例: 80%残なら80%の確率で中継)
            // さらに少し係数をかけて、残量が十分なら積極的に参加させる
            let probability = battery_percent.powf(0.5); // 平方根をとって少し甘めに
            rng.random_bool(probability as f64)
        }
    }
}

fn main() {
    println!("--- ResilientMesh v3.0: Unicorn Edition (Swarm & Disaster) ---");

    let node_count = 60;
    let mut nodes: Vec<Node> = (0..node_count).map(|i| Node::new(i)).collect();

    let bs_count = nodes.iter().filter(|n| n.node_type == NodeType::BaseStation).count();
    println!("Generated {} Nodes. (Smartphones: {}, BaseStations: {})", node_count, node_count as usize - bs_count, bs_count);

    // ネットワーク構築
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

    // パケット設定
    let start_node_id = 0;
    let target_node_id = node_count - 1;
    let mut packet_queue: VecDeque<Packet> = VecDeque::new();
    
    // 継続的にパケットを生成するためにキュー管理をループ内で行うが、
    // 今回はデモとして「生きている限りメッセージを投げ続ける」シナリオにする
    
    let mut rng = rand::rng();
    let max_steps = 40;
    let mut total_energy_consumed: f32 = 0.0;
    let mut successful_packets = 0;
    let mut disaster_triggered = false;

    // オラクル用トリガーフラグ
    let mut oracle_alert_sent = false;

    for step in 1..=max_steps {
        println!("\n[Time Step {}]", step);

        // --- 1. 災害イベント (Disaster Simulation) ---
        if step == DISASTER_STEP {
            println!("⚠️  ALERT: DISASTER OCCURRED! Huge Forest Fire in Southern Area (y < 100.0)!");
            let mut destroyed_count = 0;
            for node in &mut nodes {
                // 南側(y < 100)のノードが全滅
                if node.position.1 < 100.0 && node.is_active {
                    node.is_active = false;
                    node.battery_level = 0.0;
                    destroyed_count += 1;
                }
            }
            println!("🔥 {} nodes were destroyed immediately.", destroyed_count);
            disaster_triggered = true;
        }

        // --- 2. パケット生成 (毎ステップ、生きているStartNodeからpingを送る) ---
        if nodes[start_node_id as usize].is_active {
            packet_queue.push_back(Packet {
                id: format!("MSG_{}", step),
                history: vec![start_node_id],
                target_id: target_node_id,
                hops: 0,
                ttl: 10,
            });
        }

        // --- 3. ルーティング & バッテリー消費 ---
        // ノード待機電力
        for node in &mut nodes {
            if node.is_active {
                node.consume_battery(COST_IDLE);
                total_energy_consumed += COST_IDLE;
            }
        }

        // パケット処理
        let mut next_queue: VecDeque<Packet> = VecDeque::new();
        let mut packets_processed_this_step = 0;

        // 重複排除用 (PacketID -> Set<NodeID>)
        // 同じステップ内で同じパケットが同じノードで何度も処理されるのを防ぐ
        let mut step_visited: HashMap<String, HashSet<u32>> = HashMap::new();

        while let Some(packet) = packet_queue.pop_front() {
            let current_node_id = *packet.history.last().unwrap();

            // ゴール判定
            if current_node_id == target_node_id {
                println!("🎉 Msg '{}' REACHED GOAL via {:?} ({} hops)", packet.id, packet.history, packet.hops);
                successful_packets += 1;
                continue;
            }

            if packet.ttl == 0 { continue; }

            // 送信側チェック
            if !nodes[current_node_id as usize].is_active { continue; }

            // 送信コスト
            nodes[current_node_id as usize].consume_battery(COST_TX);
            total_energy_consumed += COST_TX;

            let peers = nodes[current_node_id as usize].peers.clone();
            
            for neighbor_id in peers {
                // 既にこのパケットが通ったノードには戻さない & このステップで処理済みならスキップ
                if packet.history.contains(&neighbor_id) { continue; }
                
                let visited_set = step_visited.entry(packet.id.clone()).or_insert(HashSet::new());
                if visited_set.contains(&neighbor_id) { continue; }

                let neighbor = &nodes[neighbor_id as usize];
                
                // 受信側が生きていて、かつ「群知能」で中継を許可するか？
                if neighbor.is_active && should_relay(neighbor, &mut rng) {
                    // 受信コスト
                    nodes[neighbor_id as usize].consume_battery(COST_RX);
                    total_energy_consumed += COST_RX;

                    let mut new_history = packet.history.clone();
                    new_history.push(neighbor_id);

                    // ログが多すぎるので間引き
                    // println!("   Forward: {} -> {}", current_node_id, neighbor_id);

                    next_queue.push_back(Packet {
                        id: packet.id.clone(),
                        history: new_history,
                        target_id: packet.target_id,
                        hops: packet.hops + 1,
                        ttl: packet.ttl - 1,
                    });
                    
                    visited_set.insert(neighbor_id);
                    packets_processed_this_step += 1;
                }
            }
        }
        packet_queue = next_queue;

        // --- 4. オラクル機能 (Proof of Disaster) ---
        // 災害発生後、しばらく成功パケットがゼロなら保険発動
        if disaster_triggered && !oracle_alert_sent {
            // ここでは簡易的に「災害後にパケット処理数が激減 or ゼロ」で判定
            // あるいは「ターゲットへの到達経路が見つからない」など
            
            // 南エリアの生存率確認
            let south_nodes = nodes.iter().filter(|n| n.position.1 < 100.0).count();
            let south_active = nodes.iter().filter(|n| n.position.1 < 100.0 && n.is_active).count();
            let survival_rate = if south_nodes > 0 { south_active as f32 / south_nodes as f32 } else { 0.0 };

            if survival_rate < 0.1 {
                 println!("\n[ORACLE] 🚨 NETWORK INTEGRITY CRITICAL: Southern Area Survival Rate {:.1}%", survival_rate * 100.0);
                 println!("[ORACLE] 💸 TRIGGER_INSURANCE_PAYOUT EVENT SENT TO ETHEREUM SMART CONTRACT");
                 println!("[ORACLE] Transaction Hash: 0x8f2d...3a1b (Simulated)\n");
                 oracle_alert_sent = true;
            }
        }
    }

    println!("\n--- v3.0 Simulation Report ---");
    println!("Total Steps: {}", max_steps);
    println!("Total Energy Consumed: {:.1}", total_energy_consumed);
    println!("Successful Packets Delivered: {}", successful_packets);
    println!("Disaster Triggered: {}", disaster_triggered);
    println!("Insurance Payout Triggered: {}", oracle_alert_sent);
}