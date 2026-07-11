use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

static CIRCUITS: OnceLock<Value> = OnceLock::new();
static BY_JOLPICA: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn jolpica_to_bacinger() -> &'static HashMap<&'static str, &'static str> {
    BY_JOLPICA.get_or_init(|| {
        HashMap::from([
            ("albert_park", "au-1953"),
            ("bahrain", "bh-2002"),
            ("shanghai", "cn-2004"),
            ("catalunya", "es-1991"),
            ("monaco", "mc-1929"),
            ("villeneuve", "ca-1978"),
            ("red_bull_ring", "at-1969"),
            ("silverstone", "gb-1948"),
            ("hungaroring", "hu-1986"),
            ("spa", "be-1925"),
            ("monza", "it-1922"),
            ("marina_bay", "sg-2008"),
            ("suzuka", "jp-1962"),
            ("americas", "us-2012"),
            ("rodriguez", "mx-1962"),
            ("interlagos", "br-1940"),
            ("yas_marina", "ae-2009"),
            ("imola", "it-1953"),
            ("zandvoort", "nl-1948"),
            ("jeddah", "sa-2021"),
            ("miami", "us-2022"),
            ("losail", "qa-2004"),
            ("baku", "az-2016"),
            ("vegas", "us-2023"),
            ("ricard", "fr-1969"),
            ("hockenheimring", "de-1932"),
            ("nurburgring", "de-1927"),
            ("portimao", "pt-2008"),
            ("mugello", "it-1914"),
            ("sepang", "my-1999"),
            ("istanbul", "tr-2005"),
            ("sochi", "ru-2014"),
            ("indianapolis", "us-1909"),
            ("kyalami", "za-1961"),
            ("estoril", "pt-1972"),
            ("galvez", "ar-1952"),
            ("jacarepagua", "br-1977"),
            ("magny_cours", "fr-1960"),
        ])
    })
}

pub fn circuits_geojson() -> &'static Value {
    CIRCUITS.get_or_init(|| {
        serde_json::from_str(include_str!("../data/circuits.geojson"))
            .expect("circuits.geojson must parse")
    })
}

pub fn geometry_for_circuit(circuit_key: &str) -> Option<Value> {
    let bacinger_id = jolpica_to_bacinger()
        .get(circuit_key)
        .copied()
        .unwrap_or(circuit_key);

    let features = circuits_geojson()
        .get("features")
        .and_then(|v| v.as_array())?;

    for feature in features {
        let id = feature
            .pointer("/properties/id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if id != bacinger_id {
            continue;
        }
        let coords = feature
            .pointer("/geometry/coordinates")
            .cloned()
            .unwrap_or(json!([]));
        let name = feature
            .pointer("/properties/Name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let location = feature
            .pointer("/properties/Location")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let length = feature
            .pointer("/properties/length")
            .cloned()
            .unwrap_or(Value::Null);

        let mut min_lng = f64::MAX;
        let mut min_lat = f64::MAX;
        let mut max_lng = f64::MIN;
        let mut max_lat = f64::MIN;
        if let Some(points) = coords.as_array() {
            for point in points {
                let lng = point.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let lat = point.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                min_lng = min_lng.min(lng);
                max_lng = max_lng.max(lng);
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
        }

        return Some(json!({
            "circuit_key": circuit_key,
            "layout_id": bacinger_id,
            "name": name,
            "location": location,
            "length_m": length,
            "type": "LineString",
            "coordinates": coords,
            "bounds": {
                "west": min_lng,
                "south": min_lat,
                "east": max_lng,
                "north": max_lat
            },
            "center": {
                "lng": (min_lng + max_lng) / 2.0,
                "lat": (min_lat + max_lat) / 2.0
            }
        }));
    }

    None
}
