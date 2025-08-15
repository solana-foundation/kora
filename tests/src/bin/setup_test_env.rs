use tests::setup_test_env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_env::run().await
}
