use axum::{
    Router,
    extract::State,
    http::header,
    response::{
        Html, IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use futures_util::stream;
use futures_util::{SinkExt, StreamExt};
use std::convert::Infallible;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

const STATE_VALUES: usize = 6 + 6 * 13;
type LatestState = watch::Receiver<Option<Vec<f64>>>;

async fn page() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../web/app.js"),
    )
}

async fn editor_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../web/editor.js"),
    )
}

async fn kinematics_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../web/kinematics.js"),
    )
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../web/style.css"),
    )
}

async fn events(
    State(latest): State<LatestState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let updates = stream::unfold((latest, false), |(mut latest, sent)| async move {
        let has_state = latest.borrow().is_some();
        if (sent || !has_state) && latest.changed().await.is_err() {
            return None;
        }

        let values = latest.borrow_and_update().clone()?;
        let json = serde_json::to_string(&values).ok()?;
        Some((Ok(Event::default().data(json)), (latest, true)))
    });
    Sse::new(updates).keep_alive(KeepAlive::default())
}

async fn receive_states(
    stream: TcpStream,
    latest: watch::Sender<Option<Vec<f64>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut socket = accept_async(stream).await?;

    while let Some(frame) = socket.next().await {
        match frame? {
            Message::Text(text) => {
                if let Some(payload) = text.as_str().strip_prefix("STATE ") {
                    match serde_json::from_str::<Vec<f64>>(payload) {
                        Ok(values)
                            if values.len() == STATE_VALUES
                                && values.iter().all(|v| v.is_finite()) =>
                        {
                            println!("{values:?}");
                            latest.send_replace(Some(values));
                        }
                        _ => eprintln!("STATE invalid: se așteaptă {STATE_VALUES} numere finite"),
                    }
                }
            }
            Message::Close(_) => {
                socket.flush().await?;
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn listen_for_robots(
    listener: TcpListener,
    latest: watch::Sender<Option<Vec<f64>>>,
) -> std::io::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let latest = latest.clone();
        tokio::spawn(async move {
            if let Err(error) = receive_states(stream, latest).await {
                eprintln!("Conexiune robot: {error}");
            }
        });
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let robots = TcpListener::bind("0.0.0.0:8080").await?;
    let browser = TcpListener::bind("0.0.0.0:8081").await?;
    let (sender, latest) = watch::channel(None);
    let app = Router::new()
        .route("/", get(page))
        .route("/app.js", get(script))
        .route("/editor.js", get(editor_script))
        .route("/kinematics.js", get(kinematics_script))
        .route("/style.css", get(stylesheet))
        .route("/events", get(events))
        .with_state(latest);

    println!("Interfața hexapod: http://localhost:8081");
    tokio::try_join!(listen_for_robots(robots, sender), axum::serve(browser, app))?;

    Ok(())
}
