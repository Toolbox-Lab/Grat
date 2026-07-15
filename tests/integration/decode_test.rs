___RUST_DOC_MOD___

#[cfg(test)]
mod tests {
    #[test]
    fn test_taxonomy_loads_all_categories() {
        let db = prism_core::taxonomy::loader::TaxonomyDatabase::load_embedded()
            .expect("Taxonomy should load");
        assert!(!db.is_empty(), "Taxonomy should contain entries");
    }

    #[test]
    fn test_diagnostic_report_serializes() {
        let report = prism_core::types::report::DiagnosticReport::new(
            "Budget",
            0,
            "LimitExceeded",
            "CPU budget exceeded",
        );
        let json = serde_json::to_string(&report).expect("Should serialize");
        assert!(json.contains("LimitExceeded"));
    }
}
