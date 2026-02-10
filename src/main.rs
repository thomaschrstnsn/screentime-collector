use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use zbus::{Connection, zvariant::OwnedValue};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// NATS server URL
    #[arg(short, long, default_value = "nats://localhost:4222")]
    nats_url: String,

    /// Comma-separated list of usernames to query and publish
    #[arg(short, long, value_delimiter = ',', num_args = 1.., required = true)]
    users: Vec<String>,

    /// Hostname where this collector is running
    #[arg(long)]
    hostname: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TimeObservation {
    #[serde(rename = "TIME_LEFT_DAY")]
    left_day: i32,
    #[serde(rename = "TIME_SPENT_BALANCE")]
    spent_balance: i32,
    #[serde(rename = "TIME_SPENT_MONTH")]
    spent_month: i32,
    #[serde(rename = "TIME_SPENT_WEEK")]
    spent_week: i32,
    #[serde(rename = "TIME_SPENT_DAY")]
    spent_day: i32,
}

impl TryFrom<&HashMap<String, OwnedValue>> for TimeObservation {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        let json_value = serde_json::to_value(value)?;
        let observation: TimeObservation = serde_json::from_value(json_value)?;
        Ok(observation)
    }
}

mod dbus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let nats_client = async_nats::connect(&args.nats_url).await?;

    let connection = Connection::session().await?;

    let proxy = dbus::timekpr::user::admin::adminProxy::new(&connection).await?;

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        for user in &args.users {
            let (_i32, _string, properties) = proxy.get_user_information(user, "").await?;
            let observation =
                TimeObservation::try_from(&properties).expect("failed to parse observation");

            let json_payload =
                serde_json::to_vec(&observation).expect("failed to serialize observation");

            let topic = format!("time.obs.{}.{}", args.hostname, user);
            let result = nats_client.publish(topic, json_payload.into()).await;

            match result {
                Ok(_) => tracing::debug!("published for user: {}", user),
                Err(e) => tracing::error!("publish failed for {}: {:?}", user, e),
            }
        }

        let result = nats_client.flush().await;
        if let Err(e) = result {
            tracing::warn!("flush failed: {:?}", e);
        } else {
            tracing::debug!("flush successful");
        }
    }
}
