use dbus::message::MatchRule;
use dbus::nonblock;
use dbus_tokio::connection;
use std::time::Duration;

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

    // Create interval - a Stream that will fire an event periodically
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    // Create a future calling D-Bus method each time the interval generates a tick
    let conn2 = conn.clone();
    // let calls = async move {
    //     loop {
    //         interval.tick().await;
    //         let conn = conn2.clone();
    //
    //         println!("Calling Hello...");
    //         let proxy = nonblock::Proxy::new(
    //             "com.timekpr.server.user.limits",
    //             "/com/timekpr/server",
    //             Duration::from_secs(2),
    //             conn,
    //         );
    //         println!("got proxy...");
    //         // TODO: Handle timeouts and errors here
    //         let (x,): (String,) = proxy
    //             .method_call("com.example.dbustest", "Hello", ("Tokio async/await",))
    //             .await
    //             .unwrap();
    //         println!("{}", x);
    //     }
    // };
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
    // calls.await;

    // ..or use the match as a stream if you prefer
    use futures_util::stream::StreamExt;
    let (incoming_signal, stream) = conn.add_match(mr).await?.stream();
    // let stream = stream.for_each(|(_, (source,)): (_, (String,))| {
    //     println!("Hello from {} happened on the bus!", source);
    //     async {}
    // });
    stream
        .for_each(|(_, (source,)): (_, (String,))| {
            println!("Hello from {} happened on the bus!", source);
            async {}
        })
        .await;
    // futures::join!(stream, calls)

    // Needed here to ensure the "incoming_signal" object is not dropped too early
    conn.remove_match(incoming_signal.token()).await?;

    unreachable!()
}
