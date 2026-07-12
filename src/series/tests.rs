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
    fn policy_blocks_paused_series() {
        let f2 = series::get("f2").expect("f2");
        assert_eq!(f2.status, SeriesStatus::Paused);
        assert!(policy::enforce_series(f2).is_err());
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
