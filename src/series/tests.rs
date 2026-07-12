#[cfg(test)]
mod tests {
    use crate::policy;
    use crate::series::{self, SeriesStatus};

    #[test]
    fn registry_lists_core_series() {
        let keys: Vec<_> = series::all()
            .into_iter()
            .map(|row| row.series_key.as_str())
            .collect();
        assert!(keys.contains(&"f1"));
        assert!(keys.contains(&"motogp"));
        assert!(keys.contains(&"wrc"));
        assert!(keys.contains(&"wec"));
        assert!(keys.len() >= 15);
    }

    #[test]
    fn newly_enabled_series_are_active() {
        for key in ["f2", "f3", "nascar-cup", "indycar"] {
            let row = series::get(key).unwrap_or_else(|| panic!("{key}"));
            assert_eq!(row.status, SeriesStatus::Active);
            assert!(policy::enforce_series(row).is_ok());
            assert!(policy::enforce_endpoint(row, "events").is_ok());
            assert!(policy::enforce_endpoint(row, "standings").is_ok());
        }
        let f2 = series::get("f2").expect("f2");
        assert!(!f2.live_enabled);
        assert!(policy::enforce_endpoint(f2, "position").is_err());
        let cup = series::get("nascar-cup").expect("nascar-cup");
        assert!(cup.live_enabled);
        assert!(policy::enforce_endpoint(cup, "position").is_ok());
    }

    #[test]
    fn policy_blocks_live_for_formula_e() {
        let fe = series::get("formula-e").expect("formula-e");
        assert!(!fe.live_enabled);
        assert!(policy::enforce_endpoint(fe, "results").is_ok());
        assert!(policy::enforce_endpoint(fe, "position").is_err());
    }

    #[test]
    fn t3_series_are_results_only() {
        let wec = series::get("wec").expect("wec");
        assert!(policy::enforce_endpoint(wec, "results").is_ok());
        assert!(policy::enforce_endpoint(wec, "position").is_err());
        assert!(policy::enforce_endpoint(wec, "laps").is_err());
    }
}
