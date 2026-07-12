use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

fn map() -> &'static HashMap<&'static str, Value> {
    static MAP: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut out = HashMap::new();
        out.insert(
            "f1",
            json!("Unofficial community project. Not associated with Formula One companies. F1 marks are trademarks of Formula One Licensing B.V."),
        );
        out.insert(
            "motogp",
            json!("Unofficial community project. Not affiliated with Dorna Sports or MotoGP, Moto2, Moto3, or MotoE rights holders."),
        );
        out.insert(
            "f2",
            json!("Unofficial community project. Not affiliated with the FIA Formula 2 Championship."),
        );
        out.insert(
            "f3",
            json!("Unofficial community project. Not affiliated with the FIA Formula 3 Championship."),
        );
        out.insert(
            "wrc",
            json!("Unofficial community project. Not affiliated with WRC Promoter GmbH or the FIA World Rally Championship."),
        );
        out.insert(
            "nascar",
            json!("Unofficial community project. Not affiliated with NASCAR."),
        );
        out.insert(
            "indycar",
            json!("Unofficial community project. Not affiliated with INDYCAR, LLC."),
        );
        out.insert(
            "formula-e",
            json!("Unofficial community project. Not affiliated with Formula E Operations."),
        );
        out.insert(
            "wec",
            json!("Unofficial community project. Not affiliated with the Automobile Club de l'Ouest or the FIA World Endurance Championship. Results-only coverage."),
        );
        out.insert(
            "imsa",
            json!("Unofficial community project. Not affiliated with IMSA. Results-only coverage."),
        );
        out.insert(
            "nls",
            json!("Unofficial community project. Not affiliated with VLN/NLS organizers. Results-only coverage."),
        );
        out.insert(
            "gtwc",
            json!("Unofficial community project. Not affiliated with SRO Motorsports Group. Results-only coverage."),
        );
        out
    })
}

pub fn text_for(key: &str) -> Value {
    map()
        .get(key)
        .cloned()
        .unwrap_or_else(|| json!("Unofficial community motorsport data project. Not affiliated with series rights holders."))
}
