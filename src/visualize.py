import json
import folium
from folium.plugins import TimestampedGeoJson

def create_map():
    print("🗺️ Loading simulation logs...")
    try:
        with open('simulation_log.json', 'r') as f:
            logs = json.load(f)
    except FileNotFoundError:
        print("❌ simulation_log.json not found. Run 'cargo run' first.")
        return

    # Center on Nice, France
    center_lat, center_lon = 43.71, 7.26
    # Premium Dark Mode Tiles
    m = folium.Map(location=[center_lat, center_lon], zoom_start=14, tiles='cartodbdark_matter')

    features = []

    # 🎨 Unicorn Neon Color Palette
    COLOR_SMARTPHONE = '#06D6A0' # Neon Green
    COLOR_BASESTATION = '#FFD166' # Neon Gold (Amber)
    COLOR_LOW_BAT = '#EF476F'     # Neon Red/Pink
    COLOR_DEAD = '#333333'        # Dark Gray
    COLOR_PACKET = '#118AB2'      # Neon Blue
    
    # Event Colors
    COLOR_DISASTER = '#D00000'    # Deep Red
    COLOR_ORACLE = '#FFD700'      # Gold

    for step_log in logs:
        step = step_log['step']
        # 1 step = 1 hour for demo purposes (starting at 10:00 AM)
        # Format: YYYY-MM-DDTHH:MM:SS
        time_str = f"2026-01-21T{10 + (step//60):02d}:{step%60:02d}:00"
        
        # Build node lookup for this step to draw packets accurately
        node_pos = {}

        # 1. NODES
        for node in step_log['nodes']:
            node_pos[node['id']] = (node['lat'], node['lon'])
            
            # Determine Color based on logic
            color = COLOR_DEAD 
            radius = 4
            opacity = 0.5
            
            if node['is_active']:
                opacity = 0.9
                if node['node_type'] == 'BaseStation':
                    color = COLOR_BASESTATION
                    radius = 8
                else:
                    color = COLOR_SMARTPHONE
                    if node['battery'] < 200:
                        color = COLOR_LOW_BAT
            
            # Create Point Feature
            features.append({
                'type': 'Feature',
                'geometry': {
                    'type': 'Point',
                    'coordinates': [node['lon'], node['lat']],
                },
                'properties': {
                    'time': time_str,
                    'icon': 'circle',
                    'iconstyle': {
                        'fillColor': color,
                        'fillOpacity': opacity,
                        'stroke': 'true',
                        'color': '#ffffff', # White border for pop
                        'weight': 1,
                        'radius': radius
                    },
                    'popup': f"Node {node['id']}<br>Type: {node['node_type']}<br>Bat: {node['battery']:.1f}"
                }
            })

        # 2. PACKETS (Traffic Flow)
        # Use 'packets' list from the log which contains successful paths
        for packet in step_log.get('packets', []):
            path_coords = []
            for nid in packet['path']:
                if nid in node_pos:
                    path_coords.append([node_pos[nid][1], node_pos[nid][0]]) # GeoJSON uses [Lon, Lat]
            
            if len(path_coords) > 1:
                features.append({
                    'type': 'Feature',
                    'geometry': {
                        'type': 'LineString',
                        'coordinates': path_coords,
                    },
                    'properties': {
                        'time': time_str,
                        'style': {
                            'color': COLOR_PACKET,
                            'weight': 3,
                            'opacity': 0.8,
                        },
                        'popup': f"Packet {packet['id']} (Hops: {len(path_coords)-1})"
                    }
                })

        # 3. EVENTS (Visualizing the Story)
        for event in step_log.get('events', []):
            if event == "DISASTER_START":
                # Large Red Pulse in the South
                 features.append({
                    'type': 'Feature',
                    'geometry': {
                        'type': 'Point',
                        'coordinates': [7.26, 43.705], # Approximate Center of Impact
                    },
                    'properties': {
                        'time': time_str,
                        'icon': 'circle',
                        'iconstyle': {
                            'fillColor': COLOR_DISASTER,
                            'fillOpacity': 0.4,
                            'stroke': 'false',
                            'radius': 60 # BIG radius
                        },
                        'popup': "⚠️ DISASTER EVENT DETECTED ⚠️"
                    }
                })
            elif event == "ORACLE_PAYOUT":
                 # Gold Pulse for Money
                 features.append({
                    'type': 'Feature',
                    'geometry': {
                        'type': 'Point',
                        'coordinates': [7.26, 43.71], # Center map
                    },
                    'properties': {
                        'time': time_str,
                        'icon': 'circle',
                        'iconstyle': {
                            'fillColor': COLOR_ORACLE,
                            'fillOpacity': 0.5,
                            'stroke': 'true',
                            'color': '#FFFFFF',
                            'weight': 3,
                            'radius': 40
                        },
                        'popup': "💸 SMART CONTRACT INSURANCE PAYOUT 💸"
                    }
                })

    print(f"🎨 Generating map with {len(features)} frames...")
    
    # Create the time-slider map
    TimestampedGeoJson(
        {'type': 'FeatureCollection', 'features': features},
        period='PT1M', # 1 Minute per step (mapped to the fake time)
        add_last_point=True,
        auto_play=True,
        loop=True,
        max_speed=1,
        loop_button=True,
        date_options='HH:mm',
        time_slider_drag_update=True
    ).add_to(m)

    m.save('map.html')
    print("✅ STARTUP_READY Map saved to 'map.html'. Open this file in your browser!")

if __name__ == "__main__":
    create_map()
