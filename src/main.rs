use anyhow::{Result, anyhow};
use serde::Serialize;
use std::{collections::HashMap, time::Duration};
use zbus::{Connection, zvariant::OwnedValue};

#[derive(Debug, Serialize)]
struct TimeObservation<T> {
    left_day: T,
    spent_balance: T,
    spent_month: T,
    spent_week: T,
    spent_day: T,
}

impl TryFrom<&HashMap<String, OwnedValue>> for TimeObservation<i32> {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, OwnedValue>) -> std::result::Result<Self, Self::Error> {
        let left_day = value
            .get("TIME_LEFT_DAY")
            .ok_or_else(|| anyhow!("bummer"))
            .map(|v| v.downcast_ref::<i32>())
            .map_err(|e| e.context("bum"))?
            .map_err(|_| anyhow!("wut"))?;

        Ok(TimeObservation {
            left_day,
            spent_balance: todo!(),
            spent_month: todo!(),
            spent_week: todo!(),
            spent_day: todo!(),
        })
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
        //
        // println!(
        //     "userid: {} full_name: {} props: {:?}",
        //     user_id, full_name, observation
        // );
        //
        // let json_payload = serde_json::to_vec(&observation).expect("lets go even more");
        //
        // let result = nats_client
        //     .publish("time.observervation.cyrus.conrad", json_payload.into())
        //     .await;
        //
        // println!("publish result: {:?}", result);
        //
        // let result = nats_client.flush().await;
        //
        // println!("flush result: {:?}", result);
    }
}
