use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::{HashSet, VecDeque};

// --- 1. 定義: ノードとパケット ---
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    id: u32,
    position: (f64, f64),
    is_active: bool,
    peers: Vec<u32>, // 接続されている近隣ノード
}

// ネットワークを流れるメッセージ
#[derive(Debug, Clone)]
struct Packet {
    id: String,
    history: Vec<u32>, // どのノードを通ってきたかの履歴（足跡）
    target_id: u32,    // 宛先
}

impl Node {
    fn new(id: u32) -> Self {
        let mut rng = rand::rng();
        Node {
            id,
            position: (rng.random_range(0.0..100.0), rng.random_range(0.0..100.0)),
            is_active: true,
            peers: Vec::new(),
        }
    }
}

// 距離計算用
fn calculate_dist(p1: (u32, f64, f64), p2: (u32, f64, f64)) -> f64 {
    let dx = p1.1 - p2.1;
    let dy = p1.2 - p2.2;
    (dx * dx + dy * dy).sqrt()
}

fn main() {
    println!("--- ResilientMesh Protocol: Simulation Start ---");

    // 1. ノードを20個生成（数を増やして網の目を密にします）
    let node_count = 20;
    let mut nodes: Vec<Node> = (0..node_count).map(|i| Node::new(i)).collect();

    // 2. ネットワーク構築（距離 35.0 以内なら接続）
    let positions: Vec<(u32, f64, f64)> = nodes.iter().map(|n| (n.id, n.position.0, n.position.1)).collect();
    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i == j { continue; }
            if calculate_dist(positions[i], positions[j]) <= 35.0 {
                nodes[i].peers.push(positions[j].0);
            }
        }
    }

    // 孤立ノードがいると実験にならないので、無理やり全員つなぐ（デモ用チート）
    // ※本来はもっと賢い配置アルゴリズムを使います
    for i in 0..nodes.len() - 1 {
        if nodes[i].peers.is_empty() {
             nodes[i].peers.push(i as u32 + 1);
             nodes[(i + 1) as usize].peers.push(i as u32);
        }
    }

    // 3. シミュレーション: Node 0 から Node (最後のID) へパケットを送る
    let start_node_id = 0;
    let target_node_id = node_count - 1; // Node 19
    
    println!("Task: Send Message from Node {} -> Node {}", start_node_id, target_node_id);

    // パケットキュー（現在ネットワーク上にあるパケット）
    let mut packet_queue: VecDeque<Packet> = VecDeque::new();
    
    // 最初のパケットを投入
    packet_queue.push_back(Packet {
        id: "MSG_001".to_string(),
        history: vec![start_node_id],
        target_id: target_node_id,
    });

    // 訪問済みリスト（無限ループ防止：同じパケットを何度も受け取らない）
    // (NodeID)
    let mut visited: HashSet<u32> = HashSet::new();
    visited.insert(start_node_id);

    // 時間ステップのループ
    let mut step = 0;
    let max_steps = 20;
    let mut success = false;

    while step < max_steps {
        step += 1;
        println!("\n[Time Step {}]", step);
        
        let mut next_queue: VecDeque<Packet> = VecDeque::new();
        let mut step_activities = 0;

        // 今あるパケットをすべて処理
        while let Some(packet) = packet_queue.pop_front() {
            let current_node_id = *packet.history.last().unwrap();
            
            // ゴール判定
            if current_node_id == target_node_id {
                println!("🎉 SUCCESS! Packet reached Goal at Step {}!", step);
                println!("🚀 Route: {:?}", packet.history);
                success = true;
                break;
            }

            // 現在のノードから、接続されている隣のノードへ拡散（Flooding）
            // ※ここで本来は「群知能」で賢く選びますが、MVPでは「全方向拡散」します
            let current_node = &nodes[current_node_id as usize];
            
            if current_node.peers.is_empty() {
                println!("   Node {} is a dead end (isolation).", current_node_id);
            }

            for &neighbor_id in &current_node.peers {
                if !visited.contains(&neighbor_id) {
                    // 新しい履歴を作る
                    let mut new_history = packet.history.clone();
                    new_history.push(neighbor_id);

                    println!("   📡 Transmission: Node {} -> Node {}", current_node_id, neighbor_id);
                    
                    next_queue.push_back(Packet {
                        id: packet.id.clone(),
                        history: new_history,
                        target_id: packet.target_id,
                    });
                    
                    visited.insert(neighbor_id);
                    step_activities += 1;
                }
            }
        }

        if success { break; }
        if step_activities == 0 {
            println!("💀 Packet died. No more paths available.");
            break;
        }

        packet_queue = next_queue;
    }

    if !success {
        println!("\n❌ FAILED. Could not reach destination.");
        println!("Network might be fragmented. Try running again (random positions).");
    }
}