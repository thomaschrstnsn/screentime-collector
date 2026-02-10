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
    left_day: i32,
    spent_balance: i32,
    spent_month: i32,
    spent_week: i32,
    spent_day: i32,
}

impl TryFrom<&HashMap<String, OwnedValue>> for TimeObservation {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        let get_i32 = |key: &str| -> anyhow::Result<i32> {
            value
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("missing key: {}", key))
                .and_then(|v| {
                    let json = serde_json::to_value(v)?;
                    Ok(serde_json::from_value(json)?)
                })
        };

        Ok(TimeObservation {
            left_day: get_i32("TIME_LEFT_DAY")?,
            spent_balance: get_i32("TIME_SPENT_BALANCE")?,
            spent_month: get_i32("TIME_SPENT_MONTH")?,
            spent_week: get_i32("TIME_SPENT_WEEK")?,
            spent_day: get_i32("TIME_SPENT_DAY")?,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_serialization() {
        let observation = TimeObservation {
            left_day: 3600,
            spent_balance: 0,
            spent_month: 18000,
            spent_week: 7200,
            spent_day: 3600,
        };

        let json_str = serde_json::to_string(&observation).expect("failed to serialize");
        let json_value: Value = serde_json::from_str(&json_str).expect("failed to deserialize");

        // Verify that keys are lowercase and values match
        assert_eq!(json_value["left_day"], 3600);
        assert_eq!(json_value["spent_balance"], 0);
        assert_eq!(json_value["spent_month"], 18000);
        assert_eq!(json_value["spent_week"], 7200);
        assert_eq!(json_value["spent_day"], 3600);
    }
}
