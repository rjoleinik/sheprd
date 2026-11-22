#![allow(clippy::unwrap_used)]
use super::request;
use http_body_util::{BodyExt as _, Full};
use hyper::{Method, Request, body::Bytes, client::conn::http1};
use hyper_util::rt::TokioIo;
use std::{future::poll_fn, task::Poll, time::Duration};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream},
    sync::oneshot,
};

async fn headers(stream: &mut DuplexStream) {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        bytes.push(stream.read_u8().await.unwrap());
        assert!(bytes.len() < 4096);
    }
}

#[tokio::test]
async fn the_next_claim_request_waits_for_http_connection_readiness() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (client, mut server) = tokio::io::duplex(4096);
        let (release, released) = oneshot::channel();
        let peer = tokio::spawn(async move {
            headers(&mut server).await;
            server
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n")
                .await
                .unwrap();
            // Keep the first response incomplete until the caller has tried
            // its next request. No scheduler timing or sleeps decide the race.
            released.await.unwrap();
            server.write_all(b"x").await.unwrap();
            headers(&mut server).await;
            server
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndone")
                .await
                .unwrap();
        });
        let (mut sender, driver) = http1::handshake(TokioIo::new(client)).await.unwrap();
        let driver = tokio::spawn(driver);
        let first = sender
            .send_request(
                Request::builder()
                    .uri("/first")
                    .body(Full::new(Bytes::new()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!sender.is_ready());
        {
            let next = request(&mut sender, Method::GET, "/next", None);
            tokio::pin!(next);
            assert!(poll_fn(|cx| Poll::Ready(next.as_mut().poll(cx).is_pending())).await);
            release.send(()).unwrap();
            let (answer, first_body) = tokio::join!(next, first.into_body().collect());
            assert_eq!(first_body.unwrap().to_bytes(), "x");
            assert_eq!(answer.unwrap(), "done");
        }
        peer.await.unwrap();
        driver.await.unwrap().unwrap();
    })
    .await
    .unwrap();
}
