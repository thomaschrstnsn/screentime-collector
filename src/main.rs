use anyhow::{Result, anyhow};
use dbus::arg::PropMap;
use dbus::arg::cast;
use dbus::message::MatchRule;
use dbus::nonblock;
use dbus_tokio::connection;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct TimeObservation<T> {
    left_day: T,
    spent_balance: T,
    spent_month: T,
    spent_week: T,
    spent_day: T,
}

trait FieldMap {
    fn get_typed_property<T>(&self, key: &str) -> anyhow::Result<T>
    where
        T: Clone,
        T: 'static;
}

impl FieldMap for PropMap {
    fn get_typed_property<T>(&self, key: &str) -> anyhow::Result<T>
    where
        T: Clone,
        T: 'static,
    {
        let variant = self
            .get(key)
            .ok_or_else(|| anyhow!("Property '{}' not found", key))?;

        cast::<T>(&variant.0)
            .cloned()
            .ok_or_else(|| anyhow!("Property '{}' has wrong type", key))
    }
}

impl<T: Clone + 'static> TryFrom<&PropMap> for TimeObservation<T> {
    type Error = anyhow::Error;
    fn try_from(value: &PropMap) -> Result<Self, Self::Error> {
        let left_day = value.get_typed_property("TIME_LEFT_DAY")?;
        let spent_balance = value.get_typed_property("TIME_SPENT_BALANCE")?;
        let spent_month = value.get_typed_property("TIME_SPENT_MONTH")?;
        let spent_week = value.get_typed_property("TIME_SPENT_WEEK")?;
        let spent_day = value.get_typed_property("TIME_SPENT_DAY")?;
        Ok(TimeObservation {
            left_day,
            spent_balance,
            spent_month,
            spent_week,
            spent_day,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the D-Bus session bus (this is blocking, unfortunately).
    let (resource, conn) = connection::new_system_sync()?;

    // The resource is a task that should be spawned onto a tokio compatible
    // reactor ASAP. If the resource ever finishes, you lost connection to D-Bus.
    //
    // To shut down the connection, both call _handle.abort() and drop the connection.
    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let nats_client = async_nats::connect("nats://enix.local:4222").await?;

    // Create interval - a Stream that will fire an event periodically
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    // Create a future calling D-Bus method each time the interval generates a tick
    let conn2 = conn.clone();
    let calls = async move {
        loop {
            interval.tick().await;
            let conn = conn2.clone();

            println!("Calling Hello...");
            let proxy = nonblock::Proxy::new(
                "com.timekpr.server",
                "/com/timekpr/server",
                Duration::from_secs(2),
                conn,
            );
            println!("got proxy...");
            // TODO: Handle timeouts and errors here
            let (user_id, full_name, properties): (i32, String, PropMap) = proxy
                .method_call(
                    "com.timekpr.server.user.admin",
                    "getUserInformation",
                    ("conrad", "F"),
                )
                .await
                .unwrap();
            let observation = TimeObservation::<i32>::try_from(&properties).expect("let go");

            println!(
                "userid: {} full_name: {} props: {:?}",
                user_id, full_name, observation
            );

            let json_payload = serde_json::to_vec(&observation).expect("lets go even more");

            let result = nats_client
                .publish("time.observervation.cyrus.conrad", json_payload.into())
                .await;

            println!("publish result: {:?}", result);

            let result = nats_client.flush().await;

            println!("flush result: {:?}", result);
        }
    };
    //
    // To receive D-Bus signals we need to add a match that defines which signals should be forwarded
    // to our application.
    let mr = MatchRule::new_signal("com.timekpr.server.user.limits", "conrad");
    // let incoming_signal = conn.add_match(mr).await?.cb(|_, (source,): (String,)| {
    //     println!("Hello from {} happened on the bus!", source);
    //     true
    // });

    // loop {
    //     println!("looping...");
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    // }

    // This will never return (except on panic) as there's no exit condition in the calls loop
    calls.await;

    // ..or use the match as a stream if you prefer
    // use futures_util::stream::StreamExt;
    // let (incoming_signal, stream) = conn.add_match(mr).await?.stream();
    // // let stream = stream.for_each(|(_, (source,)): (_, (String,))| {
    // //     println!("Hello from {} happened on the bus!", source);
    // //     async {}
    // // });
    // stream
    //     .for_each(|(_, (source,)): (_, (String,))| {
    //         println!("Hello from {} happened on the bus!", source);
    //         async {}
    //     })
    //     .await;
    // // futures::join!(stream, calls)
    //
    // Needed here to ensure the "incoming_signal" object is not dropped too early
    // conn.remove_match(incoming_signal.token()).await?;

    unreachable!()
}
