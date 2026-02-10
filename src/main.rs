use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use zbus::{Connection, zvariant::OwnedValue};

#[derive(Debug, Serialize, Deserialize)]
struct TimeObservation<T> {
    #[serde(rename = "TIME_LEFT_DAY")]
    left_day: T,
    #[serde(rename = "TIME_SPENT_BALANCE")]
    spent_balance: T,
    #[serde(rename = "TIME_SPENT_MONTH")]
    spent_month: T,
    #[serde(rename = "TIME_SPENT_WEEK")]
    spent_week: T,
    #[serde(rename = "TIME_SPENT_DAY")]
    spent_day: T,
}

impl TryFrom<&HashMap<String, OwnedValue>> for TimeObservation<i32> {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        let json_value = serde_json::to_value(value)?;
        let observation: TimeObservation<i32> = serde_json::from_value(json_value)?;
        Ok(observation)
    }
}

mod dbus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_client = async_nats::connect("nats://enix.local:4222").await?;

    let connection = Connection::session().await?;

    let proxy = dbus::timekpr::user::admin::adminProxy::new(&connection).await?;

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        let (_i32, _string, properties) = proxy.get_user_information("conrad", "").await?;
        let observation = TimeObservation::<i32>::try_from(&properties).expect("let go");

        let json_payload = serde_json::to_vec(&observation).expect("lets go even more");

        let result = nats_client
            .publish("time.obs.cyrus.conrad", json_payload.into())
            .await;

        println!("publish result: {:?}", result);

        let result = nats_client.flush().await;

        println!("flush result: {:?}", result);
    }
}
