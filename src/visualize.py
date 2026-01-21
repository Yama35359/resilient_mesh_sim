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
    m = folium.Map(location=[center_lat, center_lon], zoom_start=13)

    features = []

    for step_log in logs:
        step = step_log['step']
        # Convert step to a fake time for the slider (1 step = 1 hour for demo)
        time_str = f"2026-01-21T{10 + (step//60):02d}:{step%60:02d}:00"

        # nodes
        for node in step_log['nodes']:
            color = 'green' if node['is_active'] else 'gray'
            if node['node_type'] == 'BaseStation':
                color = 'blue'
            if node['is_active'] and node['battery'] < 200:
                color = 'orange'
            if not node['is_active']:
                color = 'black' # Dead/Destroyed

            # Create a point for each node at each step
            feature = {
                'type': 'Feature',
                'geometry': {
                    'type': 'Point',
                    'coordinates': [node['lon'], node['lat']],
                },
                'properties': {
                    'time': time_str,
                    'style': {'color': color},
                    'icon': 'circle',
                    'iconstyle': {
                        'fillColor': color,
                        'fillOpacity': 0.8,
                        'stroke': 'true',
                        'radius': 5 if node['node_type'] == 'Smartphone' else 10
                    },
                    'popup': f"Node {node['id']} ({node['node_type']})<br>Bat: {node['battery']:.1}"
                }
            }
            features.append(feature)

    print(f"🎨 Generating map with {len(features)} frames...")
    
    TimestampedGeoJson(
        {'type': 'FeatureCollection', 'features': features},
        period='PT1M',
        add_last_point=True,
        auto_play=False,
        loop=False,
        max_speed=1,
        loop_button=True,
        date_options='mm/dd HH:mm',
        time_slider_drag_update=True
    ).add_to(m)

    m.save('map.html')
    print("✅ Map saved to 'map.html'. Open this file in your browser to see the animation.")

if __name__ == "__main__":
    create_map()
