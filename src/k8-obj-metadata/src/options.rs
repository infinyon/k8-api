use serde::Serialize;

/// goes as query parameter
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListOptions {
    pub pretty: Option<bool>,
    #[serde(rename = "continue")]
    pub continu: Option<String>,
    pub field_selector: Option<String>,
    pub include_uninitialized: Option<bool>,
    pub label_selector: Option<String>,
    pub limit: Option<u32>,
    pub resource_version: Option<String>,
    pub timeout_seconds: Option<u32>,
    pub watch: Option<bool>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOptions {
    pub api_version: Option<String>,
    pub grace_period_seconds: Option<u64>,
    pub kind: Option<String>,
    pub orphan_dependents: Option<String>,
    pub preconditions: Vec<Precondition>,
    pub propagation_policy: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Precondition {
    pub uid: String,
}

#[cfg(test)]
mod test {

    use super::ListOptions;
    

    #[test]
    fn test_list_query() {
        let opt = ListOptions {
            pretty: Some(true),
            watch: Some(true),
            ..Default::default()
        };

        let qs = serde_qs::to_string(&opt).unwrap();
        assert_eq!(qs, "pretty=true&watch=true")
    }

}
