use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // subscriber helps nicely format all logs from application and middleware, and output to std out
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let config = get_configuration().expect("Could not read configuration file");
    let application = Application::build(config).await?;
    application.run_until_stopped().await?;
    Ok(())
}
