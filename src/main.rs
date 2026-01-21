use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::{HashSet, VecDeque, HashMap};

// --- 0. 定数定義 ---
// バッテリー容量 (単位: mAh 相当の抽象単位)
const BATTERY_FULL_SMARTPHONE: f32 = 1000.0;
const BATTERY_INFINITE: f32 = 999999.0; // 基地局用

// 消費コスト
const COST_IDLE: f32 = 0.5;   // 1ステップあたりの待機電力
const COST_TX: f32 = 5.0;     // パケット送信コスト
const COST_RX: f32 = 2.0;     // パケット受信コスト

// --- 1. 定義: ノードとパケット ---
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
    peers: Vec<u32>, // 接続されている近隣ノード
    
    // Extensions for Phase 1
    node_type: NodeType,
    battery_level: f32,
    transmission_range: f64,
}

// ネットワークを流れるメッセージ
#[derive(Debug, Clone)]
struct Packet {
    id: String,
    history: Vec<u32>,
    target_id: u32,
    hops: u32,
}

impl Node {
    fn new(id: u32) -> Self {
        let mut rng = rand::rng();
        
        // 10%の確率で基地局、90%でスマートフォン
        let (node_type, battery, range) = if rng.random_bool(0.1) {
            (NodeType::BaseStation, BATTERY_INFINITE, 150.0) // 基地局は遠くまで届く
        } else {
            (NodeType::Smartphone, BATTERY_FULL_SMARTPHONE, 35.0)
        };

        Node {
            id,
            position: (rng.random_range(0.0..200.0), rng.random_range(0.0..200.0)), // エリアを少し拡大
            is_active: true,
            peers: Vec::new(),
            node_type,
            battery_level: battery,
            transmission_range: range,
        }
    }

    // 距離計算用
    fn distance_to(&self, other: &Node) -> f64 {
        let dx = self.position.0 - other.position.0;
        let dy = self.position.1 - other.position.1;
        (dx * dx + dy * dy).sqrt()
    }
    
    // バッテリー消費
    fn consume_battery(&mut self, cost: f32) {
        if self.node_type == NodeType::Smartphone {
            self.battery_level = (self.battery_level - cost).max(0.0);
            if self.battery_level <= 0.0 {
                self.is_active = false;
            }
        }
    }
}

fn main() {
    println!("--- ResilientMesh Protocol v2.0: Hybrid & Energy Sim ---");

    // 1. ノード生成
    let node_count = 50; // ノード数を増やして密度を見る
    let mut nodes: Vec<Node> = (0..node_count).map(|i| Node::new(i)).collect();

    // 統計用: 基地局の数
    let bs_count = nodes.iter().filter(|n| n.node_type == NodeType::BaseStation).count();
    println!("Generated {} Nodes. (Smartphones: {}, BaseStations: {})", node_count, node_count as usize - bs_count, bs_count);

    // 2. ネットワーク構築 (非対称リンクの可能性あり)
    // A -> B が届くか？ (Aのrange内にBがいるか)
    let mut edges = 0;
    // ノードの位置を一時的に保存（借用チェッカー回避のためインデックスでアクセス）
    // Rustではベクタ内の要素同士の相互参照が少し面倒なので、IDベースで接続を構築後に適用する形にするか、
    // ここでは単純に 2重ループで index を使う。
    
    // 隣接リストを構築するための一時バッファ
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();

    for i in 0..node_count {
        adjacency.insert(i as u32, Vec::new());
    }

    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i == j { continue; }
            
            let dist = nodes[i].distance_to(&nodes[j]);
            
            // ノード i から ノード j に届くか？
            if dist <= nodes[i].transmission_range {
                adjacency.get_mut(&(i as u32)).unwrap().push(j as u32);
                edges += 1;
            }
        }
    }
    
    // ノードに適用
    for node in &mut nodes {
        if let Some(peers) = adjacency.get(&node.id) {
            node.peers = peers.clone();
        }
    }

    println!("Network constructed. Total links: {}", edges);

    // 3. シミュレーション設定
    let start_node_id = 0;
    let target_node_id = node_count - 1;
    
    println!("Task: Send Message from Node {} ({:?}) -> Node {} ({:?})", 
        start_node_id, nodes[start_node_id as usize].node_type, 
        target_node_id, nodes[target_node_id as usize].node_type);

    let mut packet_queue: VecDeque<Packet> = VecDeque::new();
    packet_queue.push_back(Packet {
        id: "MSG_001".to_string(),
        history: vec![start_node_id],
        target_id: target_node_id,
        hops: 0,
    });

    let mut visited: HashSet<u32> = HashSet::new();
    visited.insert(start_node_id);

    // シミュレーションループ
    let max_steps = 50;
    let mut step = 0;
    let mut success = false;
    let mut total_energy_consumed: f32 = 0.0;

    while step < max_steps {
        step += 1;
        println!("\n[Time Step {}]", step);
        
        let mut next_queue: VecDeque<Packet> = VecDeque::new();
        let mut step_activities = 0;

        // 全ノード待機電力消費
        for node in &mut nodes {
            if node.is_active {
                node.consume_battery(COST_IDLE);
                total_energy_consumed += COST_IDLE;
            }
        }

        while let Some(packet) = packet_queue.pop_front() {
            let current_node_id = *packet.history.last().unwrap();
            
            // ノードの参照を取得（可変でバッテリー減らすため）
            // ここではインデックスで管理しているので、ID取り出し後にノードにアクセス
            
            // ゴール判定
            if current_node_id == target_node_id {
                println!("🎉 SUCCESS! Packet reached Goal at Step {}!", step);
                println!("🚀 Route: {:?} ({} hops)", packet.history, packet.hops);
                success = true;
                break;
            }

            // 送信元のバッテリーチェック
            if !nodes[current_node_id as usize].is_active {
                println!("   Node {} is dead (Battery 0%). Dropping packet.", current_node_id);
                continue;
            }

            let peers = nodes[current_node_id as usize].peers.clone();
            
            // 送信コスト消費
            nodes[current_node_id as usize].consume_battery(COST_TX);
            total_energy_consumed += COST_TX;

            for neighbor_id in peers {
                if !visited.contains(&neighbor_id) {
                    // 受信側のチェック
                    if nodes[neighbor_id as usize].is_active {
                        // 受信コスト消費
                        nodes[neighbor_id as usize].consume_battery(COST_RX);
                        total_energy_consumed += COST_RX;

                        let mut new_history = packet.history.clone();
                        new_history.push(neighbor_id);

                        println!("   📡 Node {} -> Node {} (Bat: {:.1})", 
                            current_node_id, neighbor_id, nodes[neighbor_id as usize].battery_level);
                        
                        next_queue.push_back(Packet {
                            id: packet.id.clone(),
                            history: new_history,
                            target_id: packet.target_id,
                            hops: packet.hops + 1,
                        });
                        
                        visited.insert(neighbor_id);
                        step_activities += 1;
                    }
                }
            }
        }

        if success { break; }
        if step_activities == 0 && next_queue.is_empty() {
             println!("💀 Packet died. No more paths available.");
             break;
        }

        packet_queue = next_queue;
    }

    println!("\n--- Result Report ---");
    if success {
        println!("Status: SUCCESS");
    } else {
        println!("Status: FAILED");
    }
    println!("Total Energy Consumed: {:.1} units", total_energy_consumed);
    
    // 残存スマートフォン平均バッテリー
    let smart_nodes: Vec<&Node> = nodes.iter().filter(|n| n.node_type == NodeType::Smartphone).collect();
    let total_bat: f32 = smart_nodes.iter().map(|n| n.battery_level).sum();
    let avg_bat = if !smart_nodes.is_empty() { total_bat / smart_nodes.len() as f32 } else { 0.0 };
    println!("Avg Smartphone Battery Remaining: {:.1}/{:.1}", avg_bat, BATTERY_FULL_SMARTPHONE);
}