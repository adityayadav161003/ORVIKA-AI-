pub trait CloudProvider: Send + Sync {
    async fn execute_query(&self, query: &str, api_key: &str) -> Result<String, String>;
}
